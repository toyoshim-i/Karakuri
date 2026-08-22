//! A control surface for a model rather than for a pair of hands.
//!
//! `--mcp PORT` serves the Model Context Protocol over HTTP on the loopback
//! interface, so a chat client can read a slot's procedure, rewrite it, and be
//! told what the compiler and the frame budget made of the result. The name for
//! what that enables is **vibe live coding**: "the one that is showing now, a
//! bit more vivid" is a small edit to a declarative file, and the file is the
//! thing this system was already built to hot-swap.
//!
//! ## It is the third surface, and it obeys the same rule as the other two
//!
//! Keys, then MIDI, now this. `README.md`'s invariant is that everything an
//! operator moves goes through a record, is read back, and only then applied —
//! and `docs/roadmap.md` already demanded exactly this of M6's agents, in those
//! words, before anyone had thought about MCP:
//!
//! > The record stream must be the sole mutation path, so that an agent is
//! > structurally incapable of doing anything a human could not do through the
//! > same interface
//!
//! An agent from outside the process is still an agent — and this surface is
//! the reason the invariant is now true of *material* as well as of the mix.
//! It was not: a procedure change was a file and not a record, so a session in
//! which a model rewrote slot 0 at minute ten replayed with the procedure it
//! started with, silently. The hole predated this module by as long as
//! `--watch` has existed, and nobody was going to be misled by a human typing
//! in vim; somebody would certainly have been misled by this. `Record::Procedure`
//! closes it, so **a session driven by a model does replay with no model
//! attached**, and what an agent did during a set can be watched back.
//!
//! ## Most of this never touches the frame
//!
//! Reading a procedure is reading a file. Writing one is checking it and
//! writing a file — the compile, the frame-boundary swap, the thirty measured
//! frames and the rollback if it costs too much are `--watch`'s, built for
//! editing by hand and now doing the most dangerous part of this: **a model
//! that writes something too expensive is caught by the machinery that already
//! catches a human who does.**
//!
//! ## Two channels, and neither of them is a lock
//!
//! The outcome of a swap comes from the render loop over a channel rather than
//! a lock, so the frame path neither blocks nor waits. `save_set` is the first
//! tool that needs the *other* direction: what a slot is playing lives on the
//! render thread and is reachable from nowhere else, so a request goes back the
//! same way, is taken where the MIDI surface is taken, and ends in the method
//! the `k` key ends in. **There is one save path in this program**, and it is
//! `Live::save_set`; this is a way to ask for it and not a second copy of it.
//!
//! **The wait for its answer happens with [`State`] unlocked.** There is a
//! thread per connection and one mutex over the state, so a tool that waited
//! for a disk while holding it would stop every other connection — including
//! one that only wanted to read a procedure — for as long as the loop took.
//! [`Pending`] exists for no other reason.
//!
//! ## Loopback only
//!
//! There is no bind option and there should not be one. A venue network is
//! shared, and a port that can rewrite what is on the projector is not
//! something to expose by a flag anyone might pass without meaning it. Reaching
//! a render machine from a laptop is `ssh -L`, which is a thing an operator
//! does deliberately and can see.

use std::io::{BufRead, Read, Write};
use std::sync::mpsc;

use karakuri_ir::Kind;
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{Layer, Record};
use karakuri_store::store::{Store, StoreError};
use serde_json::{json, Value};

/// What the render loop tells the server about, over a channel.
///
/// **A channel and not a shared lock**: the frame path may wait for nothing,
/// and a swap is rare enough that the send costs less than the `eprintln!`
/// beside it already does.
pub enum Event {
    /// A build landed, was rolled back, or failed, in the words the operator
    /// saw on the terminal.
    Swap { slot: usize, said: String },
}

/// **A save one client is asking the render loop for.**
///
/// The channel this travels on is the one this module did not have, and the
/// reason a fourth tool cost more than the first three: reading and writing a
/// procedure are files, and this is the *live Set* — which only the render
/// thread holds. So the request goes to the loop and the answer comes back,
/// rather than this module growing a second idea of what a slot is playing.
pub struct SaveRequest {
    /// Which deck slot. Checked against [`Slots`] before it is sent, so a slot
    /// this deck does not hold meets the refusal `read_procedure` gives it; the
    /// loop checks the range again, because it is the only thing that knows how
    /// many slots the *deck* has.
    pub slot: usize,
    /// What to file it under, or `None` to let the loop name it after the
    /// moment — which is what a key press gets, for the reason
    /// [`crate::history::stamped_id`] states: a key cannot type a name.
    pub id: Option<String>,
    /// Where the answer goes.
    pub reply: Reply,
}

/// **The half of one [`SaveRequest`] the render loop answers on.**
///
/// Two messages rather than one, because the loop already says two things: that
/// it has started a save, at the frame it was asked, and what became of it, at
/// the frame the disk answered. A client still waiting when the deadline passes
/// has the first of them — which is what makes a truthful timeout possible at
/// all, since "accepted, under this id, outcome not yet known" is a different
/// fact from either success or failure.
pub struct Reply(mpsc::Sender<News>);

impl Reply {
    /// The loop has taken it and named the set, in the words it printed.
    ///
    /// **Never blocks**: the channel is unbounded and one save puts at most two
    /// messages in it. This is called from a frame.
    pub fn accepted(&self, said: &str) {
        let _ = self.0.send(News::Accepted(said.to_string()));
    }

    /// What the save came to, in the words the operator was given for it.
    ///
    /// Sent from the frame the outcome landed on, which is the frame the `save`
    /// record is written on — so what a model is told and what the stream says
    /// come from one place. `Err` is a refusal or a disk that said no, and both
    /// reach the client as a failed tool call.
    pub fn settled(self, said: Result<String, String>) {
        let _ = self.0.send(News::Settled(said));
    }
}

/// What a [`Reply`] carries, in the order it carries it.
enum News {
    Accepted(String),
    Settled(Result<String, String>),
}

/// The half of the server the render loop holds.
pub struct Reporter {
    sender: mpsc::SyncSender<Event>,
    /// What clients have asked the loop to do. The receiving half, because this
    /// is the direction [`Event`] does not go in.
    requests: mpsc::Receiver<SaveRequest>,
    /// Reports the queue had no room for. **Counted rather than lost quietly**:
    /// a client that is told what happened must be told when it is not the
    /// whole story.
    dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
    port: u16,
}

impl Reporter {
    /// Tell the server what a slot's swap machinery just did.
    ///
    /// **Never blocks and never waits for room.** A bounded queue and
    /// `try_send`: a client that has stopped asking must not be able to make
    /// the render thread wait, and the queue in front of the bounded history
    /// used to be unbounded, so it was not a report on the present at all.
    pub fn swap(&self, slot: usize, said: &str) {
        let event = Event::Swap {
            slot,
            said: said.to_string(),
        };
        if self.sender.try_send(event).is_err() {
            self.dropped
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// **Every save a client has asked for since this was last called.**
    ///
    /// Drained and never waited on, from the frame path, exactly as the MIDI
    /// surface is drained beside it: a frame owes a client nothing, and each
    /// request ends in the same method the `k` key ends in. Empty on almost
    /// every frame, and an empty `collect` allocates nothing.
    pub fn saves(&self) -> impl Iterator<Item = SaveRequest> + '_ {
        self.requests.try_iter()
    }

    /// The port actually bound, which is not the one asked for when that was 0.
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Where a slot's procedures live, so a tool can name a slot rather than a
/// path.
///
/// **Paths never cross the protocol.** A client may be on another machine
/// through an `ssh -L`, where a path means nothing — and a tool that took one
/// would be inviting a model to write anywhere on the render machine's disk.
#[derive(Clone)]
pub struct Slots(pub Vec<(std::path::PathBuf, Vec<std::path::PathBuf>)>);

impl Slots {
    /// A slot's files, each under the layer and the index the rest of this
    /// program addresses it by.
    ///
    /// **The layer is read off the file, not off its position.** The first path
    /// is the slot's L1 — that is what `main.rs` loads it as, before it has
    /// looked at a `kind` at all — and every later one is on the layer its own
    /// `kind` line names, at its position *within that layer*, keeping file
    /// order. That is the rule `history::seed` files snapshots under and the
    /// rule the startup path sorts a `--set` chain by, and it is
    /// [`crate::history::declared_kind`] here rather than a second scanner:
    /// two readers of a `kind` line would be two answers to what layer a file
    /// is on, and the layer a version is filed under has to be the layer an
    /// agent addresses it by.
    ///
    /// A text scan and not a parse, for that function's reason: this surface
    /// works without compiling anything, and **a file that does not compile
    /// must still be addressable** — it is the one a model most needs to read.
    /// One that cannot be read, or that declares no `kind`, is counted as a
    /// renderer, which is what `history::seed` makes of it and what the compile
    /// is about to refuse it as.
    ///
    /// Read on every call rather than worked out once at startup. The files are
    /// a handful and nothing here is on a frame path, and a layout cached
    /// beside a directory `--watch` is editing is a layout that can be wrong.
    fn nodes(&self, slot: usize) -> Result<Vec<(Kind, usize, &std::path::PathBuf)>, String> {
        let pair = self
            .0
            .get(slot)
            // **The one sentence, from [`crate::no_such_slot`].** This used to
            // be its own spelling — `this deck holds 0-3` against the keys'
            // `this deck holds slots 0-3` — so a model calling `save_set` and
            // an operator pressing a digit were told the same mistake in
            // different words about the same control. It also handled the empty
            // deck for its own reason, which that function has too: `len() - 1`
            // underflowed on an empty deck and took the whole surface with it,
            // the panic unwinding out of the listener thread so that a process
            // which had announced a port was silently no longer on it. `serve`
            // refuses an empty deck now; this stays correct anyway.
            .ok_or_else(|| crate::no_such_slot(slot, self.0.len()))?;
        let mut nodes = vec![(Kind::L1, 0, &pair.0)];
        // The next free index per layer, which the head has already taken one
        // of: a `--set` chain naming a second `kind L1` is a second source, and
        // it is L1 number 1 rather than the beginning of a fresh count.
        let mut next: Vec<(Kind, usize)> = vec![(Kind::L1, 1)];
        for path in &pair.1 {
            let layer = std::fs::read(path)
                .ok()
                .and_then(|source| crate::history::declared_kind(&source))
                .and_then(layer_named)
                .unwrap_or(Kind::L4);
            let index = match next.iter_mut().find(|(held, _)| *held == layer) {
                Some((_, free)) => {
                    let index = *free;
                    *free += 1;
                    index
                }
                None => {
                    next.push((layer, 1));
                    0
                }
            };
            nodes.push((layer, index, path));
        }
        Ok(nodes)
    }

    /// **Whether this deck holds `slot` at all, without reading a byte off
    /// disk.**
    ///
    /// [`Slots::nodes`] answers this too, and costs a `read` per file of the
    /// slot to do it, because it works each node's layer out of the file's own
    /// `kind` line. `save_set` wants nothing but the range and was calling
    /// `nodes` for it — **under the state mutex**, which is the one lock in this
    /// server a slow disk can be held across, and it is held across every other
    /// connection's request as well. The number was `self.0.len()` all along.
    fn holds(&self, slot: usize) -> Result<(), String> {
        if slot < self.0.len() {
            Ok(())
        } else {
            Err(crate::no_such_slot(slot, self.0.len()))
        }
    }

    /// The file one `(slot, layer, index)` address names.
    fn path(&self, slot: usize, layer: Kind, index: usize) -> Result<&std::path::PathBuf, String> {
        let nodes = self.nodes(slot)?;
        if let Some((_, _, path)) = nodes.iter().find(|(l, i, _)| *l == layer && *i == index) {
            return Ok(*path);
        }
        // **What the slot holds, rather than "no such node".** An index past
        // the end and a layer this slot does not use are different mistakes,
        // and a model told which one it made can fix its own call — the same
        // reason the checker's diagnostics come back through here instead of
        // going to a terminal nobody is watching.
        let name = layer_name(layer);
        Err(match nodes.iter().filter(|(l, _, _)| *l == layer).count() {
            0 => format!("slot {slot} holds no {name}: {}", absent(layer)),
            1 => format!("slot {slot} holds one {name} and `index` is {index}"),
            n => format!(
                "slot {slot} holds {n} {name} nodes, so `index` is 0-{}",
                n - 1
            ),
        })
    }
}

/// Every layer a slot's files can be on, in the order they compose.
///
/// **One list, so the enum a client is handed, the address this resolves and
/// the `kind` a written source must declare cannot disagree.** They did: the
/// schema offered `L1` and `L4` alone for as long as a slot could hold five
/// kinds of node, so the most interesting material in the language — the
/// deformations, the camera, the field — was in the files and unreachable from
/// the one surface built for editing them.
const LAYERS: [Kind; 5] = [Kind::L1, Kind::L2, Kind::L3, Kind::L4, Kind::Field];

/// A layer as a client writes it, in the compiler's own `Kind`.
///
/// Case-folded because `l1` is what a model tends to type and refusing it
/// teaches nobody anything. `Field` is spelled as `--param` and `--bind` spell
/// it, which is as the `kind` line does.
fn layer_named(name: &str) -> Option<Kind> {
    Some(match name.to_ascii_uppercase().as_str() {
        "L1" => Kind::L1,
        "L2" => Kind::L2,
        "L3" => Kind::L3,
        "L4" => Kind::L4,
        "FIELD" => Kind::Field,
        _ => return None,
    })
}

/// The name back again, for a schema and for a sentence.
///
/// Exhaustive on purpose: a sixth `Kind` should not compile until somebody has
/// decided what this surface calls it and whether [`LAYERS`] offers it.
fn layer_name(layer: Kind) -> &'static str {
    match layer {
        Kind::L1 => "L1",
        Kind::L2 => "L2",
        Kind::L3 => "L3",
        Kind::L4 => "L4",
        Kind::Field => "Field",
    }
}

/// The layers, as a client is told them in a refusal.
fn layer_list() -> String {
    LAYERS
        .iter()
        .map(|layer| layer_name(*layer))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What it means for a slot to hold none of a layer, which is a different thing
/// for each of them: three are optional and two cannot be missing.
fn absent(layer: Kind) -> &'static str {
    match layer {
        // Unreachable, because a slot's first path is its L1 whatever it says.
        // Written out anyway: the arm that cannot happen is the one that stops
        // saying so quietly when the shape around it changes.
        Kind::L1 => "which cannot happen — a slot's first file is its geometry",
        Kind::L2 => {
            "a deformation is optional, and one is added by naming its file in the same \
             `--set` chain"
        }
        Kind::L3 => "a camera is optional, and a slot without one looks from the built-in orbit",
        Kind::L4 => "a Set needs at least one renderer",
        Kind::Field => {
            "a `kind Field` is optional, and is code the other procedures evaluate rather \
             than a node of its own"
        }
    }
}

/// The largest request body this will read.
///
/// **Checked before anything is allocated**, which is not tidiness: `length`
/// arrives from whoever opened the socket, and `vec![0u8; length]` on a number
/// it cannot serve calls `handle_alloc_error`, which **aborts the process**. It
/// is not catchable and it is not confined to this thread — a fifty-six byte
/// request line took the projector out mid-set. A procedure is a few kilobytes.
const MAX_BODY: usize = 1 << 20;

/// How long one connection may say nothing before it is dropped.
///
/// A crashed client, a browser keep-alive, or a socket that connects and waits
/// used to hold the *only* server thread for the rest of the run. There is a
/// thread per connection now as well, and this is the second half of that fix:
/// threads are cheap, but not if they accumulate forever.
const IDLE: std::time::Duration = std::time::Duration::from_secs(30);

/// How many swap reports may queue for a client that is not asking.
///
/// The queue used to be unbounded, which made "a report on the present" false
/// of everything in front of the bounded part: two hundred thousand events, each
/// holding a `String` allocated on the render thread, materialised in one drain.
/// Dropped rather than queued past this, and **counted**, because a report with
/// a hole in it must say so.
const QUEUED: usize = 256;

/// How many save requests may be waiting for the render loop at once.
///
/// [`QUEUED`]'s trade in the other direction, and the same one: a bound, and a
/// refusal rather than a wait when it is reached. The loop takes every request
/// it has on the next frame, so this is only ever full when the loop has
/// stopped running frames — which is a thing to *answer*, because a client
/// queued behind a loop that will never take its request would wait forever.
const ASKED: usize = 16;

/// The longest an `id` a client names may be.
///
/// It becomes a file name under `<store>/sets/`, and a stamp is twenty
/// characters.
const MAX_ID: usize = 64;

/// **How long a `save_set` call waits for the render loop before it answers
/// without an outcome.**
///
/// The run's own bound on a save is [`crate::SAVE_WAIT`] — how long quitting
/// will wait for a disk that is not answering — and this is that, plus room for
/// the frame that takes the request and the frame that reports it back. Under
/// the run's own bound it would give up on saves the run itself would still
/// have finished, which is the one number this must not be below.
const SAVE_REPLY: std::time::Duration =
    std::time::Duration::from_secs(crate::SAVE_WAIT.as_secs() + 5);

/// Serve MCP on `port`, loopback only, until the process ends.
///
/// Returns the [`Reporter`] the render loop keeps. The listener and everything
/// behind it live on threads of their own; nothing here is ever called from a
/// frame.
///
/// **`store` is a root and not an open [`Store`]**, which is the same decision
/// [`Slots`] makes about a path and for a milder version of the same reason.
/// Opening here would fail a whole run for a library nothing has asked for yet,
/// and would hold one answer to "where is the store" against a directory the
/// operator is free to move; opening per call is four `create_dir_all`s off a
/// frame path, on a surface where the expensive thing is already a compile.
pub fn serve(
    port: u16,
    slots: Slots,
    store: std::path::PathBuf,
    watching: bool,
) -> Result<Reporter, String> {
    if slots.0.is_empty() {
        return Err("this run has no procedure files to serve — see `--load-set`".into());
    }
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("port {port}: {e}"))?;
    // Asked back rather than echoed: `--mcp 0` binds an ephemeral port, and
    // printing the 0 tells the operator a port that is not the port.
    let bound = listener
        .local_addr()
        .map_err(|e| format!("port {port}: {e}"))?;
    let (tx, rx) = mpsc::sync_channel(QUEUED);
    // The other direction, made here for the same reason: the render loop is
    // handed one half of everything it shares with this server, once, before a
    // frame has run.
    let (asked, requests) = mpsc::sync_channel(ASKED);

    let state = std::sync::Arc::new(std::sync::Mutex::new(State {
        slots,
        store,
        watching,
        events: rx,
        asked,
        recent: Vec::new(),
        dropped: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
    }));
    let dropped = state.lock().expect("fresh mutex").dropped.clone();

    std::thread::Builder::new()
        .name("mcp".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let state = state.clone();
                // **A thread per connection.** One thread for the listener and
                // every conversation meant a socket that said nothing wedged
                // the surface for the rest of the run — and a panic inside it
                // dropped the listener, leaving a process that had announced a
                // port and was no longer on it.
                let spawned =
                    std::thread::Builder::new()
                        .name("mcp-conn".into())
                        .spawn(move || {
                            if let Err(e) = handle(stream, &state) {
                                eprintln!("mcp: {e}");
                            }
                        });
                if spawned.is_err() {
                    eprintln!("mcp: could not start a thread for a connection");
                }
            }
        })
        .map_err(|e| format!("starting the mcp thread: {e}"))?;

    Ok(Reporter {
        sender: tx,
        requests,
        dropped,
        port: bound.port(),
    })
}

struct State {
    slots: Slots,
    /// **Where the library lives** — `--store DIR`, the same root every other
    /// half of this run reads and writes. A root rather than an open [`Store`];
    /// see [`serve`].
    store: std::path::PathBuf,
    /// Whether `--watch` is on. Without it a written procedure sits on disk and
    /// changes nothing, which a model has no way to discover and every reason
    /// to be told.
    watching: bool,
    events: mpsc::Receiver<Event>,
    /// Where a save a client asks for goes. **Bounded and never blocked on** —
    /// see [`ASKED`]: this is sent into from a connection thread, and a render
    /// loop that has stopped taking requests must produce an answer rather than
    /// a thread that never returns.
    asked: mpsc::SyncSender<SaveRequest>,
    /// What the swap machinery has said, newest last, bounded.
    recent: Vec<String>,
    dropped: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

/// How much swap history is kept. Enough for a model to see what its own last
/// write did and no more: this is a report on the present, not a log.
const RECENT: usize = 32;

impl State {
    fn drain(&mut self) {
        while let Ok(Event::Swap { slot, said }) = self.events.try_recv() {
            self.recent.push(format!("slot {slot}: {said}"));
        }
        if self.recent.len() > RECENT {
            self.recent.drain(..self.recent.len() - RECENT);
        }
    }
}

// -- the transport ---------------------------------------------------------

/// One connection: read requests, answer them, keep it open.
///
/// **Hand-rolled, and what that costs is worth stating rather than assuming.**
/// The first version argued it was fine because this is "loopback, from one
/// client". Loopback is not a boundary — any process on the machine reaches it,
/// and so does a `fetch()` from any web page the operator happens to have open,
/// because a POST with a plain content type needs no preflight. "One client" was
/// an assumption about the *good* client and nothing enforced it. What enforces
/// anything now: an `Origin` check, a body cap before any allocation, a read
/// timeout, and a thread per connection.
fn handle(stream: std::net::TcpStream, state: &std::sync::Mutex<State>) -> Result<(), String> {
    stream.set_nodelay(true).ok();
    stream.set_read_timeout(Some(IDLE)).ok();
    let mut reader = std::io::BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut writer = stream;

    loop {
        let mut line = String::new();
        if read_capped(&mut reader, &mut line)? == 0 {
            return Ok(()); // the client hung up
        }
        let mut request_line = line.split_whitespace();
        let method = request_line.next().unwrap_or("").to_string();
        // **The path, which the first version threw away.** It answered 405 to
        // every request whatever it asked for, and a client's auth discovery
        // asks for `/.well-known/oauth-protected-resource` before it does
        // anything else. 405 says "that exists, but not by this verb", so the
        // client concluded there was protected-resource metadata to fetch and
        // went looking for it — then failed parsing `this server only answers
        // POST` as JSON. The whole handshake died on a path this server has
        // never had.
        let target = request_line.next().unwrap_or("").to_string();

        let mut length: Option<usize> = None;
        let mut origin: Option<String> = None;
        loop {
            let mut header = String::new();
            if read_capped(&mut reader, &mut header)? == 0 {
                return Ok(());
            }
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            let Some((name, value)) = header.split_once(':') else {
                // A line with no colon is not a header. Refusing the whole
                // request beats guessing what was meant.
                return respond(&mut writer, 400, "text/plain", b"malformed header");
            };
            // **Case-insensitively**, because field names are, and because
            // `CONTENT-LENGTH:` is lawful and used to be read as a body of zero
            // — after which the client was told its JSON was malformed and its
            // bytes were reparsed as headers.
            match name.trim().to_ascii_lowercase().as_str() {
                "content-length" => match value.trim().parse::<usize>() {
                    Ok(n) => length = Some(n),
                    Err(_) => {
                        return respond(
                            &mut writer,
                            400,
                            "text/plain",
                            b"content-length is not a number",
                        )
                    }
                },
                "origin" => origin = Some(value.trim().to_string()),
                _ => {}
            }
        }

        // **The check the first version did not have.** A page on any site can
        // POST here; it cannot read the reply, and `write_procedure` does not
        // need to be read to have happened. A client that is genuinely local
        // sends no `Origin` at all.
        if let Some(origin) = &origin {
            if !is_local_origin(origin) {
                return respond(
                    &mut writer,
                    403,
                    "text/plain",
                    b"this server does not answer cross-origin requests",
                );
            }
        }

        // **Path before method**, because "no such thing here" and "not by that
        // verb" are different answers and only one of them is true of a path
        // this server does not serve. A 404 is what tells a client there is no
        // authorization metadata to find, which is how a server with no auth
        // says so.
        let path = target.split(['?', '#']).next().unwrap_or("");
        if path != ENDPOINT {
            return respond(
                &mut writer,
                404,
                "application/json",
                br#"{"error":"no such path: this server serves MCP at / and nothing else"}"#,
            );
        }

        if method != "POST" {
            // 405 rather than 404 *here*: the path is real, and the Streamable
            // HTTP transport says a server offering no SSE stream at its
            // endpoint answers GET with exactly this. Answered and closed
            // rather than answered and continued: the body of a non-POST was
            // left in the reader, so it became the next request line and ran.
            // Closing cannot be smuggled through.
            //
            // JSON rather than plain text because a client that reached here
            // is a client parsing JSON — the same reason the 404 above carries
            // a body it can read.
            return respond(
                &mut writer,
                405,
                "application/json",
                br#"{"error":"this endpoint answers POST only: there is no SSE stream here"}"#,
            );
        }

        let Some(length) = length else {
            return respond(
                &mut writer,
                411,
                "text/plain",
                b"a POST needs a content-length",
            );
        };
        if length > MAX_BODY {
            return respond(&mut writer, 413, "text/plain", b"that body is too large");
        }

        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;

        let reply = match serde_json::from_slice::<Value>(&body) {
            Ok(request) => {
                let mut state = state.lock().map_err(|_| "the mcp state is poisoned")?;
                let pending = dispatch(&request, &mut state);
                // **The lock, let go before anything is waited for.** There is
                // a thread per connection and one mutex over the state, so a
                // `save_set` that waited for the render loop and the disk here
                // would hold up every other connection for as long as it took —
                // including a client that only wanted to read a procedure, and
                // including the client that would have asked what the swap did.
                // Dropped by name rather than by a scope, because a scope is a
                // thing somebody widens later without noticing what it was for.
                drop(state);
                pending.settled()
            }
            Err(e) => Some(error(&Value::Null, -32700, &format!("parse error: {e}"))),
        };
        match reply {
            Some(reply) => {
                let bytes = serde_json::to_vec(&reply).map_err(|e| e.to_string())?;
                respond(&mut writer, 200, "application/json", &bytes)?;
            }
            // A notification: answered with 202 and no body, which is what the
            // protocol asks for.
            None => respond(&mut writer, 202, "text/plain", b"")?,
        }
    }
}

/// A header or request line, refused past [`MAX_BODY`] rather than grown.
///
/// `read_line` has no cap, so a line with no newline in it is a second way to
/// exhaust memory — quieter than a huge `Content-Length` and the same ending.
fn read_capped(reader: &mut impl BufRead, into: &mut String) -> Result<usize, String> {
    let mut taken = std::io::Read::take(reader.by_ref(), MAX_BODY as u64);
    let read = taken
        .read_line(into)
        .map_err(|e| format!("reading a header: {e}"))?;
    if read >= MAX_BODY {
        return Err("a request line was longer than this server will read".into());
    }
    Ok(read)
}

/// Whether an `Origin` is one this server will answer.
///
/// Only the loopback names, and only because a browser sends `Origin` on every
/// cross-site request while a real client sends none at all.
fn is_local_origin(origin: &str) -> bool {
    let rest = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .unwrap_or(origin);
    let host = rest.split(':').next().unwrap_or("");
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

fn respond(
    writer: &mut std::net::TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    // The reason, not "OK" after every code. `HTTP/1.1 405 OK` was on the wire.
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
        body.len()
    );
    writer
        .write_all(head.as_bytes())
        .map_err(|e| e.to_string())?;
    writer.write_all(body).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}

// -- the protocol ----------------------------------------------------------

/// The version of MCP this speaks.
const PROTOCOL: &str = "2024-11-05";

/// **A reply, or the one step of a reply that must happen with [`State`]
/// unlocked.**
///
/// This type is the whole of the concurrency design, so it is worth stating
/// plainly what it buys. `handle` locks the state around [`dispatch`], and there
/// is a thread per connection: anything waited for under that lock is waited for
/// by every other client too. Four of the five tools are a file read or a file
/// write and finish under it — `read_set` is the widest of them, a set file and
/// a card for each node it names, and that is a bounded count of reads off the
/// store rather than a wait on anybody else's thread; `save_set` waits for a
/// render loop and then for a disk, which is unbounded in the only sense that
/// matters — it depends on somebody else's frame rate.
///
/// So the send happens under the lock, where the channel is, and the *wait*
/// comes back out here. Returning a value that still has work in it is the
/// smallest thing that makes the boundary visible: a comment saying "do not
/// wait here" would be a comment.
enum Pending {
    /// Nothing left to do. `None` is a notification, which is answered with no
    /// body at all.
    Done(Option<Value>),
    /// A save the render loop has been asked for, and the JSON-RPC id it is
    /// answered under.
    Saving {
        id: Value,
        news: mpsc::Receiver<News>,
    },
}

impl Pending {
    /// The reply, waiting for the render loop if that is what is left.
    ///
    /// **Called with the state unlocked**, which is the entire reason this type
    /// exists — see above.
    fn settled(self) -> Option<Value> {
        match self {
            Pending::Done(reply) => reply,
            Pending::Saving { id, news } => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": tool_result(awaited(&news, SAVE_REPLY)),
            })),
        }
    }
}

fn dispatch(request: &Value, state: &mut State) -> Pending {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    // **No `id` is a notification: never answered, and this server acts on none
    // of them.** The comment here used to say "acted on", which was false —
    // nothing below this line runs — and the test asserting `is_none()` could
    // not tell the difference. There is nothing a notification asks of this
    // server today; when there is, it goes above this line.
    let Some(id) = id else {
        return Pending::Done(None);
    };

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL,
            "capabilities": { "tools": {}, "resources": {} },
            "serverInfo": { "name": "karakuri", "version": env!("CARGO_PKG_VERSION") },
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "resources/list" => Ok(json!({ "resources": resources() })),
        "resources/read" => read_resource(request).map_err(Refused::BadParams),
        "tools/call" => match call_tool(request, state) {
            // **Out from under the lock before it is waited for.** See
            // [`Pending`]; this `return` is the only thing carrying that
            // decision, so it is the one line here worth reading twice.
            Ok(Called::Saving(news)) => return Pending::Saving { id, news },
            Ok(Called::Answered(outcome)) => Ok(tool_result(outcome)),
            Err(e) => Err(Refused::BadParams(e)),
        },
        other => Err(Refused::NoMethod(format!("no method `{other}`"))),
    };

    Pending::Done(Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        // **The code says which kind of wrong.** Everything used to come back
        // as "method not found", so a client could not tell a method it had
        // invented from arguments it had got wrong.
        Err(Refused::NoMethod(m)) => error(&id, -32601, &m),
        Err(Refused::BadParams(m)) => error(&id, -32602, &m),
    }))
}

/// Why a request could not be answered, in the two shapes JSON-RPC has codes
/// for.
enum Refused {
    NoMethod(String),
    BadParams(String),
}

fn error(id: &Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

// -- tools -----------------------------------------------------------------

fn tools() -> Value {
    // The layers, from [`LAYERS`] rather than written out beside it. A client
    // is offered exactly what [`layer_named`] accepts and what [`Slots::path`]
    // resolves, because it is the same list — the drift this closes is the one
    // that left three of a slot's five layers unaddressable while the files
    // were sitting right there.
    let layers: Vec<&str> = LAYERS.iter().map(|layer| layer_name(*layer)).collect();
    json!([
        {
            "name": "read_procedure",
            "description":
                "The source of one node of one deck slot. `layer` says which: L1 is what \
                 the elements are and how they move, L2 a deformation applied to them, L3 \
                 the camera, L4 how they are drawn, and Field a distance function the \
                 others evaluate. Read before writing: the edit is usually small, and what \
                 is already there is the best guide to the language.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": { "type": "integer", "description": "deck slot, from 0" },
                    "layer": { "type": "string", "enum": layers },
                    "index": {
                        "type": "integer",
                        "description":
                            "which node of that layer, from 0, in the order the slot's files \
                             were named. A slot draws with as many L4s as it likes — the same \
                             cloud as sprites and as strokes is one slot with two — and may \
                             simulate with more than one L1, look from more than one L3 and \
                             hold more than one Field. Omit for the first.",
                    },
                },
                "required": ["slot", "layer"],
            },
        },
        {
            "name": "write_procedure",
            "description":
                "Check a procedure and, if it compiles, write it. It is then compiled on a \
                 worker thread, swapped in at a frame boundary, and measured for thirty \
                 frames — if it costs more than the frame budget it is dropped and the \
                 previous one comes back at the time it was parked at. So an expensive \
                 mistake is survivable and a non-compiling one never reaches the screen. \
                 **The diagnostics are the point of the return value**: if it does not \
                 compile, what comes back is what the checker said, against the source.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": { "type": "integer" },
                    "layer": { "type": "string", "enum": layers },
                    "index": {
                        "type": "integer",
                        "description":
                            "which node of that layer, from 0. Omit for the first. The \
                             source's own `kind` line must name the same layer as this \
                             address, which is what stops a deformation being written over \
                             a renderer.",
                    },
                    "source": { "type": "string", "description": "the whole procedure" },
                },
                "required": ["slot", "layer", "source"],
            },
        },
        {
            "name": "swap_outcome",
            "description":
                "What the swap machinery has said recently: whether a written procedure \
                 landed, was rolled back for cost, or failed to build. Call it after a \
                 write to find out what happened — a write returning cleanly means it \
                 compiled, not that it is on screen.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "save_set",
            "description":
                "Keep what a slot is playing, as a Set file that can be loaded again with \
                 `--load-set ID`. It writes the material **on screen** — the versions the \
                 slot is running, by content hash, with the parameters, capacities and \
                 salts the live Set holds now — and not what any file on disk says, which \
                 is exactly what the operator's `k` key writes. That distinction is the \
                 point: a procedure that was written and then rolled back for cost is on \
                 disk and not on screen, and this saves the screen. **It waits for the \
                 disk and tells you what happened**, so what comes back names the id it \
                 was saved under; do not report a set as kept until it does.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": { "type": "integer", "description": "deck slot, from 0" },
                    "id": {
                        "type": "string",
                        "description":
                            "what to file it under: letters, digits, `-` and `_`, and it \
                             becomes a file name. **An id that already names a set is \
                             overwritten**, as `--save-set ID` overwrites — a name you \
                             choose is an instruction and nothing is renamed behind you, so \
                             use a new one for each keeper. Omit it and the set is named \
                             after the moment it was saved, which is what the key press \
                             gets — a name an operator can find by the time they saved it, \
                             and one that cannot collide.",
                    },
                },
                "required": ["slot"],
            },
        },
        {
            "name": "read_set",
            "description":
                "What a saved Set holds, and what each procedure in it declares — read \
                 out of the library without loading anything and without compiling \
                 anything. A Set is a slot's material kept under a name: `save_set` \
                 writes one, so does the operator's `k` key, and `--load-set ID` plays \
                 one back. For every node this says which layer it is on — L1 is what \
                 the elements are and how they move, L2 a deformation, L3 the camera, \
                 L4 how they are drawn, Field a distance function the others evaluate — \
                 what the procedure calls itself, and what it *declares*: each \
                 parameter with the two numbers a value must lie between and the value \
                 it takes when nothing turns it; the element count an L1 may run at, \
                 lowest, highest and the count it runs at unless a Set says otherwise; \
                 and the attributes it emits, which are what a renderer drawn over it \
                 can consume. **Ranges are declarations, not settings**: a range says \
                 what a value will be refused outside of, not where this Set has it. \
                 Call it to choose between things you have kept, and to find out what \
                 there is to turn on one, without fetching its source and compiling it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description":
                            "the id the set was filed under: letters, digits, `-` and \
                             `_`. It is what `save_set` came back naming, what \
                             `--save-set ID` was given, or the stamp a save that named \
                             nothing was called after.",
                    },
                },
                "required": ["id"],
            },
        },
    ])
}

/// What one tool call came to: an answer, or a wait that belongs outside the
/// state lock. See [`Pending`].
enum Called {
    Answered(Result<String, String>),
    Saving(mpsc::Receiver<News>),
}

fn call_tool(request: &Value, state: &mut State) -> Result<Called, String> {
    let params = request.get("params").ok_or("no params")?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("no tool name")?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    Ok(match name {
        "read_procedure" => Called::Answered(read_procedure(&args, state)),
        "write_procedure" => Called::Answered(write_procedure(&args, state)),
        "swap_outcome" => Called::Answered(swap_outcome(state)),
        // Answered here like a read and unlike `save_set`: a card is a file, the
        // render loop does not hold one, and there is nothing to wait for.
        "read_set" => Called::Answered(read_set(&args, state)),
        // **Refused here and waited for elsewhere.** Everything this module can
        // decide by itself — a slot that does not exist, an `id` that is not a
        // name — is decided under the lock like any other tool's arguments, and
        // only the wait for somebody else's thread is deferred.
        "save_set" => match save_set(&args, state) {
            Ok(news) => Called::Saving(news),
            Err(refusal) => Called::Answered(Err(refusal)),
        },
        other => return Err(format!("no tool `{other}`")),
    })
}

/// One tool call's answer, in the shape the protocol gives a tool.
///
/// **A tool failure is a result, not a protocol error.** A model that is told
/// "the call was malformed" learns nothing; one handed the checker's
/// diagnostics can fix its own source, which is the whole loop.
///
/// A function rather than a `json!` at each call site, because `save_set`'s
/// answer is built after the lock is gone — see [`Pending`] — and two spellings
/// of this shape would be two chances to disagree about `isError`.
fn tool_result(outcome: Result<String, String>) -> Value {
    match outcome {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        Err(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": true }),
    }
}

/// **`index` is optional and defaults to 0**, unlike the wildcard an absent
/// `index` means on a `param` record. The difference is the same one that runs
/// through the whole address: this names *a procedure to read or rewrite*, and
/// there is no such thing as rewriting every renderer at once with one source
/// — where a `param` addresses a *value*, and one value reaching every
/// declaration is both meaningful and the useful default.
///
/// It defaults because the first node of a layer is what a client that says
/// nothing means, on every layer: a slot holding one camera and one shape has
/// nothing else `index` could name, and one holding two has an order its files
/// were given in.
///
/// **The layer is parsed here into the compiler's own `Kind`** and travels as
/// one from here on, so the layer this resolves a file for and the layer a
/// written source is checked against are the same value rather than two
/// readings of one string.
fn slot_layer_index(args: &Value) -> Result<(usize, Kind, usize), String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let named = args
        .get("layer")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`layer` is required and is one of {}", layer_list()))?;
    let layer = layer_named(named)
        .ok_or_else(|| format!("no layer `{named}`: a slot's nodes are {}", layer_list()))?;
    let index = match args.get("index") {
        None => 0,
        Some(v) => v
            .as_u64()
            .ok_or("`index` is a number: which node of that layer, from 0")?
            as usize,
    };
    Ok((slot, layer, index))
}

fn read_procedure(args: &Value, state: &State) -> Result<String, String> {
    let (slot, layer, index) = slot_layer_index(args)?;
    let path = state.slots.path(slot, layer, index)?;
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_procedure(args: &Value, state: &State) -> Result<String, String> {
    let (slot, layer, index) = slot_layer_index(args)?;
    let source = args
        .get("source")
        .and_then(Value::as_str)
        .ok_or("`source` is required")?;
    let path = state.slots.path(slot, layer, index)?.clone();
    let name = layer_name(layer);

    // **Checked before it is written, and the diagnostics are handed back.**
    // Writing first and letting the watcher report would put the compiler's
    // answer on a terminal the model cannot see.
    let checked = crate::compile::check(source)?;
    // **The address and the source have to agree**, and the comparison is now
    // between two `Kind`s rather than between a string and a guess. The guess
    // was `L1`, or `L4` for everything else, which made this refusal answer
    // about a layer nobody had named: a `kind L2` sent to a slot's L2 was
    // turned away for not being a renderer, which is a refusal about a mistake
    // the caller had not made.
    if checked.kind != layer {
        return Err(format!(
            "this is a {:?} procedure and it was addressed to slot {slot}'s {name} — \
             the two layers are not interchangeable, and what a file is is the `kind` \
             line inside it",
            checked.kind
        ));
    }

    // **What else this write reaches.** `watch.rs` documents two slots sharing a
    // pair as supported, and the manual's own example gives one `soft_points.kir`
    // to three slots — so naming one slot was reporting a third of what
    // happened. Anything skipped or widened is said with a count.
    //
    // **Every node of every other slot**, whatever layer it is on: the scan
    // walked an L1 and a list of renderers, which is the shape a slot had
    // before it could hold a deformation chain — so one `swirl_warp.kir` given
    // to two slots was a write that silently changed both and named one.
    let also: Vec<String> = (0..state.slots.0.len())
        .filter(|other| *other != slot)
        .filter(|other| {
            state
                .slots
                .nodes(*other)
                .is_ok_and(|nodes| nodes.iter().any(|(_, _, held)| **held == path))
        })
        .map(|other| other.to_string())
        .collect();

    std::fs::write(&path, source).map_err(|e| format!("{}: {e}", path.display()))?;
    let shared = if also.is_empty() {
        String::new()
    } else {
        format!(
            " This file is also slot{} {}, which now show the same procedure.",
            if also.len() == 1 { "" } else { "s" },
            also.join(", ")
        )
    };
    // Named as an address rather than as a layer, because two renderers or two
    // sources are only told apart by the index — the same `layer:index:`
    // `--param` writes.
    Ok(if state.watching {
        format!(
            "compiled and written to slot {slot} {name}:{index}.{shared} It is being built on a \
             worker thread and will swap in at a frame boundary; call `swap_outcome` to \
             find out whether it landed or was rolled back for cost.\n\n\
             This replaced the file on disk. The version it replaced is in the run's edit \
             history under `<store>/history/`, where every version that compiled is kept — \
             so it can be got back, but not from here."
        )
    } else {
        format!(
            "compiled and written to slot {slot} {name}:{index}.{shared} **This run was started \
             without `--watch`, so nothing will pick it up** — the file has changed and the \
             screen has not. It replaced the file on disk and there is no backup."
        )
    })
}

fn swap_outcome(state: &mut State) -> Result<String, String> {
    state.drain();
    let dropped = state.dropped.load(std::sync::atomic::Ordering::Relaxed);
    let missing = if dropped == 0 {
        String::new()
    } else {
        format!(
            "\n\n({dropped} earlier report{} were dropped for want of room — this is \
                 not the whole history)",
            if dropped == 1 { "" } else { "s" }
        )
    };
    Ok(if state.recent.is_empty() {
        format!(
            "nothing has swapped, rolled back or failed to build since this run \
             started.{missing}"
        )
    } else {
        format!("{}{missing}", state.recent.join("\n"))
    })
}

/// Ask the render loop to keep what a slot is playing.
///
/// **`slot` is required and `id` is not**, which is [`slot_layer_index`]'s
/// convention and its reason: an argument a client says nothing about should
/// mean the obvious thing, and the obvious thing here is the name a key press
/// gets. Unlike `index` there is no default written down — the loop stamps it,
/// and stamping it here would be a second answer to what a nameless save is
/// called.
///
/// Nothing about the Set is read here and nothing could be: this thread does not
/// hold it. What this does is check what it can check, hand the request over,
/// and give the caller back the half it waits on.
fn save_set(args: &Value, state: &State) -> Result<mpsc::Receiver<News>, String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    // **The refusal `read_procedure` gives for a slot this deck does not hold,
    // from the same place.** The loop checks the range again — it is the only
    // thing that knows the *deck*'s slot count — but a model that asked for slot
    // 9 is owed the answer now rather than after a round trip. See
    // [`Slots::holds`] on why this is not [`Slots::nodes`].
    state.slots.holds(slot)?;
    let id = match args.get("id") {
        // **`null` is absent, not a bad string.** A client that builds its
        // arguments from a record with an empty field sends `"id": null`, and
        // that is a caller saying nothing about the id rather than one getting
        // its type wrong — "`id` is a string" is a refusal about a mistake it
        // did not make. Every other optional argument here reads an absent one
        // as its default; `null` is the second spelling of absent and gets the
        // same answer. It is *not* the same as `""`, which is a caller naming a
        // file with no name and is still refused — see [`checked_id`].
        None | Some(Value::Null) => None,
        Some(id) => Some(checked_id(
            id.as_str()
                .ok_or("`id` is a string: what to file the set under")?,
        )?),
    };

    let (tx, rx) = mpsc::channel();
    state
        .asked
        .try_send(SaveRequest {
            slot,
            id,
            reply: Reply(tx),
        })
        // **Answered rather than waited for**, both ways. A model must never be
        // left holding a call on a loop that will not answer it, and these are
        // the two shapes of "it will not": one that has stopped taking requests,
        // and one that is gone.
        .map_err(|e| match e {
            // **What `Full` proves and no more.** It used to say the loop "has
            // taken none of them", which the error does not support: the queue
            // holds [`ASKED`] requests nobody has taken *yet*, and a loop
            // running slowly reaches that as surely as one that has stopped.
            // Naming the second as though it were the fact would send a model
            // looking for a dead render thread when the answer is to ask again.
            mpsc::TrySendError::Full(_) => format!(
                "the render loop has {ASKED} save requests queued and no room for another: \
                 it is taking them slower than they are arriving, or it is not running \
                 frames at all. Nothing was saved, and asking again is safe"
            ),
            mpsc::TrySendError::Disconnected(_) => {
                "the render loop has ended: this run is shutting down and nothing was saved"
                    .to_string()
            }
        })?;
    Ok(rx)
}

/// **What one saved Set holds, and what each of its procedures declares** —
/// off the store, with nothing loaded, nothing compiled and no GPU.
///
/// **This is the reader `<hash>.meta.ndjson` did not have.** Every path that
/// stores an artifact from a compile writes a card beside it — see
/// [`crate::meta::card`] — and until this, `Store::read_meta` had no caller
/// outside its own tests. A figure with a producer and no consumer is how the
/// last wrong number in this program got published, so the card gets its reader
/// in the same milestone that gave it a writer.
///
/// **A tool and not a resource.** `docs/roadmap.md`, M4: *"the resource list is
/// a curriculum, not an index"* — a resource is a curated few a client reads in
/// full, and a user's Sets are neither curated nor few nor knowable at startup.
/// The two resources here are the spec and the vocabulary, which every client
/// should read once; a library is searched, and searching is a call.
///
/// **The caller names a Set, because a Set id is the only handle a model can
/// hold.** The three candidates were the address the other tools take
/// (`slot`, `layer`, `index`), a Set id, and a content hash:
///
/// - **A hash is what the card is filed under and it is the one to reject**,
///   easiest though it is. Nothing in this protocol has ever handed a model a
///   hash, so the first call could not be made — a tool whose argument only
///   this tool's own output can supply is a tool nobody can start using. It is
///   also the *only* one of the three that needs no validation, being hex and
///   64 characters, and choosing an argument for the convenience of its
///   validation is choosing the wrong argument.
/// - **`(slot, layer, index)` names what is on screen**, whose source a model
///   can already fetch with `read_procedure` and read the declarations off
///   directly. It would answer a question that is already answerable.
/// - **A Set id names material this surface is otherwise blind to.** A saved
///   Set that has not been loaded has no file behind it that `read_procedure`
///   can reach — its sources are bytes in the store under hashes nothing shows
///   — so *"which of these saved things should I use"* is unanswerable without
///   this. `save_set` comes back naming the id it wrote, and `--load-set ID`
///   is spelled with one, so a model that has kept anything has one in hand.
///
/// **What this does not say is what the Set has those knobs turned to.** The
/// file read here carries `param` and `capacity` records beside the `slot`s and
/// they are deliberately passed over: a `param_decl` says a knob exists and
/// what it may be turned between, a `param` says where this Set left it, and
/// the record vocabulary keeps them apart under two names for exactly that
/// reason. Rendering both in one block would be the place they get confused,
/// and *"what is it set to"* is a second question that deserves being asked as
/// one. The lines are in hand the moment anybody wants it.
fn read_set(args: &Value, state: &State) -> Result<String, String> {
    let id = args
        .get("id")
        .and_then(Value::as_str)
        .ok_or("`id` is required and is a string: which set to read")?;
    // **The same check `save_set` puts a name through, and the reason is the
    // same one.** This id becomes `<store>/sets/<id>.set.ndjson`, so
    // `../../../somewhere/else` is a path, and paths never cross this protocol —
    // see [`checked_id`] and [`Slots`]. A read is not the harmless half of that
    // rule: it is the half that hands a file's contents back to the caller.
    let id = checked_id(id)?;
    let store = Store::open(&state.store)
        .map_err(|e| format!("the store at `{}`: {e}", state.store.display()))?;
    let lines = store.read_set(&id).map_err(|e| {
        format!(
            "reading set `{id}`: {e} — a set is filed under the id it was saved \
             under, by `save_set`, by the operator's `k` key or by `--save-set ID`, \
             and this store holds only the ones written into it"
        )
    })?;
    // **The file's own order**, which is the order [`crate::setfile::save`]
    // wrote the nodes in, and the order a hand-written file chose. Sorting by
    // layer would impose a reading nobody wrote, for the reason
    // [`crate::meta::card`] keeps a procedure's parameters in declaration order.
    let nodes: Vec<(Layer, u32, Option<String>, Hash)> = lines
        .iter()
        .filter_map(|line| match line.record() {
            Record::Slot {
                layer,
                index,
                name,
                proc_hash,
            } => Some((*layer, *index, name.clone(), *proc_hash)),
            _ => None,
        })
        .collect();
    if nodes.is_empty() {
        return Ok(format!(
            "set `{id}` is in this store and names no material: it holds {} record{} \
             and none of them is a `slot`, so there is nothing in it to describe.",
            lines.len(),
            if lines.len() == 1 { "" } else { "s" },
        ));
    }
    let mut out = format!(
        "set `{id}` holds {} node{}, and `--load-set {id}` plays it. Everything below \
         is what a procedure *declares* — the range a value is refused outside of — \
         and not what this set has anything turned to.\n",
        nodes.len(),
        if nodes.len() == 1 { "" } else { "s" },
    );
    for (layer, index, name, hash) in nodes {
        out.push('\n');
        out.push_str(&node_block(&store, layer, index, name.as_deref(), &hash));
    }
    Ok(out)
}

/// One node of a Set: its address in the Set, its artifact, and its card.
fn node_block(store: &Store, layer: Layer, index: u32, name: Option<&str>, hash: &Hash) -> String {
    // **Twelve hex characters and not sixty-four.** A hash is not an argument
    // anything here takes — see [`read_set`] — so what this is for is telling
    // two nodes apart and recognising the same artifact in two Sets, which
    // twelve does at a length a reader can hold. `Hash::short` is the same
    // shortening every log line in this program uses.
    let stored = format!("stored as {}", hash.short(12));
    let called = match name {
        Some(name) => format!(", called `{name}` in this set"),
        None => String::new(),
    };
    let address = format!("{}:{index}", layer_spelled(layer));
    match store.read_meta(hash) {
        Ok(card) => {
            let (declared, body) = rendered_card(&card);
            let declared = declared.map_or(String::new(), |name| format!(" `{name}`"));
            format!("{address}{declared} — {stored}{called}\n{body}")
        }
        // **Not an error, and it must not read as one.** `Store::read_meta`
        // answers `NotFound` for a card that was never written, which is an
        // ordinary state of a working store rather than damage: a card is
        // derived, `Store::put_artifact` writes none of its own — it takes bytes
        // and does not compile — and an artifact stored before cards existed has
        // none either. A model told "not found" would report a broken library;
        // what it is owed is the sentence that says the source is there and the
        // description is not.
        //
        // **Two absences, and the pair is worth the extra read.** A hash with no
        // card and a hash this store has never seen are the same `NotFound` from
        // here and completely different facts: the second means the Set was
        // written against another store and will not load here at all, which is
        // the more useful thing anyone could be told and is invisible if both
        // say "no card". The artifact is only fetched on this branch, so the
        // ordinary path pays nothing for it.
        Err(StoreError::NotFound(_)) => {
            let standing = if store.get_artifact(hash).is_err() {
                "this store does not hold that artifact at all, so nothing here can \
                 say what it declares and `--load-set` could not build this set \
                 either — the set was saved somewhere else, or beside a store that \
                 has since been moved"
            } else {
                "its source is here and it has no metadata card. That is an ordinary \
                 state and not a damaged store: a card is derived rather than kept, so \
                 an artifact stored as bytes, or stored by a build older than cards, \
                 has none until something compiles it and stores it again. What it \
                 declares is in its source, at the top of the procedure"
            };
            format!("{address} — {stored}{called}\n  {standing}.\n")
        }
        // A card that is there and will not read is the one case that *is* a
        // damaged store, and it says so in different words for that reason.
        Err(e) => format!("{address} — {stored}{called}\n  its card could not be read: {e}\n"),
    }
}

/// A card's four records as prose: what the procedure calls itself, and the
/// lines describing what it declares.
///
/// **Only the four a card can carry today.** `origin`, `parent`, `perf`, `tag`
/// and `thumbnail` are specified and nothing writes one — see
/// [`crate::meta::card`], which says why each is absent rather than empty — so
/// they fall through the catch-all, which is also what makes this reader survive
/// meeting a card written by a build that has more of them.
fn rendered_card(card: &[Line]) -> (Option<String>, String) {
    let mut declared = None;
    let mut body = String::new();
    let mut params = 0usize;
    for line in card {
        match line.record() {
            Record::Meta { name, .. } => declared = Some(name.clone()),
            Record::ParamDecl {
                key,
                ty,
                min,
                max,
                default,
            } => {
                params += 1;
                body.push_str(&format!(
                    "  param {key} : {ty}, anywhere from {min} to {max}{}\n",
                    match default {
                        Some(default) => format!(", and {default} until something turns it"),
                        // The record's own reading, in words: an absent
                        // `default` says the default is not a number this build
                        // can state, never that there is none — every declared
                        // param has one, because the `.kir` grammar makes the
                        // expression mandatory.
                        None => ". Its default is an expression rather than a literal, so the \
                             card cannot state it as a number"
                            .to_string(),
                    }
                ));
            }
            Record::CapacityDecl { min, max, default } => body.push_str(&format!(
                "  capacity: between {min} and {max} elements, and {default} of them \
                 until a set says otherwise\n"
            )),
            Record::Emit { attrs } => body.push_str(&format!(
                "  emits {} — what a renderer drawn over it can consume\n",
                attrs.join(", ")
            )),
            _ => {}
        }
    }
    // Said rather than left to silence: a block with no `param` line reads as a
    // rendering that dropped them. Every other absence here is a whole record
    // the card deliberately does not write — see [`crate::meta::card`] — and
    // reads correctly as nothing, but "there is nothing to turn on this one" is
    // an answer to the question that was asked.
    if params == 0 {
        body.push_str("  no parameters: there is nothing to turn on this one\n");
    }
    (declared, body)
}

/// A record [`Layer`] under the name this protocol already spells it with.
///
/// **Found through [`crate::setfile::layer_of`] rather than matched again.**
/// The mapping between a record's `Layer` and the compiler's `Kind` exists once,
/// is total, and is the one `--load-set` reads a Set through; a second match
/// here would be a second answer to which layer a stored node is on, and the
/// name a model is given for a node has to be the name it addresses one by. The
/// fallback cannot be reached while that mapping stays total — and it renders as
/// a word rather than panicking, because a layer added on one side only is a
/// thing to see in an answer, not a thread to take down.
fn layer_spelled(layer: Layer) -> &'static str {
    LAYERS
        .iter()
        .copied()
        .find(|kind| crate::setfile::layer_of(*kind) == layer)
        .map_or("unknown", layer_name)
}

/// A Set id a client may name, or why not.
///
/// **A Set id is one path component.** [`crate::history::stamped_id`] says so
/// where it explains why the date is spelled `20260816` rather than
/// `2026/08/16`, and `Store::set_path` spells the file `sets/<id>.set.ndjson`
/// without checking that what it was handed is one. That is the operator's own
/// business on `--save-set`, where the id came out of their own shell. It is not
/// a model's: this is the same rule [`Slots`] exists for — **paths never cross
/// the protocol** — and `../../../somewhere/else` is a path.
///
/// Letters, digits, `-` and `_`, which is what a stamp is made of and what a
/// name anybody would type is made of. **Refused rather than sanitised**: a set
/// filed under a name its caller did not ask for is a worse answer than one that
/// is told to pick another.
///
/// **A name a client picks twice overwrites, and that is the decision rather
/// than an oversight.** [`crate::history::unused`] exists because two saves in
/// one millisecond produced one stamp and the second file replaced the first
/// while the operator was told both were kept, and its own doc names this
/// control as the reach that would make that matter. It is not reached from
/// here, and it must not be: it renames — `keeper` becomes `keeper-1` — which is
/// exactly the sanitising the paragraph above refuses, and it would rename only
/// inside one run, so the same call in tomorrow's run would overwrite anyway.
/// A *stamp* is a name nobody chose and renaming one loses nothing; a name a
/// caller typed is an instruction, and `--save-set ID` has always obeyed it by
/// overwriting. So `save_set` does what `--save-set` does, and says so — in the
/// tool description a model reads, in `docs/manual.md` and in
/// `docs/roadmap.md`. Undocumented was the thing that was not allowed.
///
/// **No `con`, `nul`, `aux`, `com1` check.** They are reserved device names on
/// Windows and would be a file that is not a file. There is no Windows target
/// today and no `cfg` for one here; this sentence is the record that the case is
/// known, so that whoever ports this finds it written down rather than finds it
/// on a projector.
fn checked_id(id: &str) -> Result<String, String> {
    if id.is_empty() {
        return Err(
            "`id` is empty: a set is filed under a name, or under none at all if \
                    `id` is left out"
                .into(),
        );
    }
    // **Bytes, and the message says bytes.** `str::len` is bytes and this said
    // "characters", which is the same number for everything that gets past the
    // charset check below and a different one for what does not — so the one
    // caller the message existed for, the one sending something this refuses,
    // was told a number it could not count to.
    if id.len() > MAX_ID {
        return Err(format!(
            "`id` is {} bytes and the most is {MAX_ID}: it becomes a file name",
            id.len()
        ));
    }
    if let Some(bad) = id
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
    {
        return Err(format!(
            "`id` holds `{bad}`, and a set id is letters, digits, `-` and `_`: it is one \
             path component and it names a file under `<store>/sets/`"
        ));
    }
    Ok(id.to_string())
}

/// **Wait for one save's outcome, and say something true when it does not
/// come.**
///
/// A free function over the channel rather than a loop inside [`Pending`], for
/// the reason `main.rs`'s `drained_saves` is one: the bound is the whole of what
/// makes waiting here safe, and it has to be checkable without a render loop, a
/// window or a disk.
///
/// **It waits, rather than returning on acceptance, and that was the decision
/// worth arguing.** Answering the moment the loop has the request would make
/// this tool cheap and its answer worthless: a model told "saved" before the
/// disk has spoken will tell its user the set is kept, and the cases where that
/// is a lie — a full store, a network mount that stopped answering, a slot whose
/// sources are not savable — are precisely the ones anybody would want to hear
/// about. The other three tools already work this way: `write_procedure` hands
/// back the checker's verdict and not "it is being checked".
///
/// **A timeout is neither success nor failure, and the text says so.** The
/// protocol has one boolean and it cannot carry a third state, so `isError` is
/// set — a model reading `isError: false` reports the set as kept, which is the
/// one thing that must not happen here, while a model reading `true` looks
/// again. What the flag cannot carry, the sentence does.
fn awaited(news: &mpsc::Receiver<News>, wait: std::time::Duration) -> Result<String, String> {
    let deadline = std::time::Instant::now() + wait;
    let mut accepted: Option<String> = None;
    while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
        match news.recv_timeout(left) {
            Ok(News::Accepted(said)) => accepted = Some(said),
            Ok(News::Settled(outcome)) => return outcome,
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            // **The loop dropped the request without answering it**, which is
            // what the end of a run looks like from here. Which of the two
            // sentences depends on whether it was ever taken: one that was
            // never taken saved nothing, and one that was may well have reached
            // the disk on the way out.
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(match accepted {
                    Some(said) => format!(
                        "{said}\n\nThe render loop then ended without saying what became of \
                         it. Whether that file was written is not something this server can \
                         still find out."
                    ),
                    None => "the render loop ended before it took this save: nothing was saved"
                        .to_string(),
                })
            }
        }
    }
    Err(match accepted {
        Some(said) => format!(
            "{said}\n\n**This is neither a success nor a failure.** The save was accepted \
             and had not reported back after {wait:?}. It is being written or it is not; \
             nothing here knows which, and no `save` record claims either way until it \
             lands. Do not report the set as kept — look for it under that id."
        ),
        None => format!(
            "the render loop had not taken this save after {wait:?} — it is running slowly \
             or not at all. Nothing was saved, and asking again is safe."
        ),
    })
}

// -- resources -------------------------------------------------------------

/// The one path this server serves. Everything else is a 404, which is what
/// tells a client probing for authorization metadata that there is none.
const ENDPOINT: &str = "/";

const SPEC: &str = "karakuri://ir-spec";
const VOCABULARY: &str = "karakuri://ir-vocabulary";

fn resources() -> Value {
    json!([
        {
            "uri": SPEC,
            "name": "The IR specification",
            "description":
                "The language a procedure is written in, in full, with the reasoning. \
                 Read this before writing a procedure for the first time.",
            "mimeType": "text/markdown",
        },
        {
            "uri": VOCABULARY,
            "name": "Built-in functions the checker accepts",
            "description":
                "Every built-in with its signature, generated from the checker's own \
                 table rather than written down beside it. Prose drifts from code; this \
                 cannot, because the same list is what rejects a procedure.",
            "mimeType": "text/markdown",
        },
    ])
}

fn read_resource(request: &Value) -> Result<Value, String> {
    let uri = request
        .get("params")
        .and_then(|p| p.get("uri"))
        .and_then(Value::as_str)
        .ok_or("no uri")?;
    let text = match uri {
        SPEC => include_str!("../../../docs/ir-spec.md").to_string(),
        VOCABULARY => vocabulary(),
        other => return Err(format!("no resource `{other}`")),
    };
    Ok(json!({
        "contents": [{ "uri": uri, "mimeType": "text/markdown", "text": text }],
    }))
}

/// The built-ins, rendered from [`karakuri_ir::builtin::Builtin::ALL`].
///
/// **Generated, and that is the whole point.** `docs/ir-spec.md` describes this
/// language in prose and prose goes stale; this list is the one the checker
/// matches against, so it cannot say a function exists that does not, or miss
/// one that does.
fn vocabulary() -> String {
    use karakuri_ir::builtin::Builtin;
    use karakuri_ir::{Blend, Output, Topology};
    let mut out = String::from(
        "# Built-in functions\n\n\
         Generated from the checker's own table, so this is exactly what will be \
         accepted.\n\n\
         `Same` means the argument takes the shape of the others; `Scalar` is a single \
         float; `Exact(T)` is that type and no other. `domain` says whether a function \
         is defined on floats and vectors alike, on vectors only, or on one concrete \
         shape.\n\n\
         | name | arguments | returns | domain | must be constant |\n\
         |---|---|---|---|---|\n",
    );
    for builtin in Builtin::ALL {
        let signature = builtin.signature();
        let args = signature
            .args
            .iter()
            .map(|a| format!("{a:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let constants = if signature.const_args.is_empty() {
            String::new()
        } else {
            format!("{:?}", signature.const_args)
        };
        out.push_str(&format!(
            "| `{}` | {args} | {:?} | {:?} | {constants} |\n",
            builtin.name(),
            signature.ret,
            signature.domain
        ));
    }

    // Generated for the same reason the table above is: these are closed
    // vocabularies in the checker, so a hand-written list here could name a
    // topology or an output that does not exist. What cannot be generated is
    // which outputs are *required* — that is a rule in the check pass rather
    // than a property of the enum — so the prose says it and the spec resource
    // carries the detail.
    out.push_str(
        "\n# Topologies\n\nDeclared by an L1's `topology`. What a *renderer* draws \
                  is not declared: an L4 draws segments when its `vertex` block assigns \
                  `clip_b` and sprites when it does not.\n\n",
    );
    for topology in [Topology::Points, Topology::Lines, Topology::Fullscreen] {
        let note = match topology {
            Topology::Points => "one sprite per element",
            Topology::Lines => "one segment per element, `clip` to `clip_b`",
            // Listed with the rule it brings rather than only with what it
            // draws, because `consumes` is checked rather than merely expected
            // and a model that did not know would meet the refusal after
            // writing the file. An earlier version of this arm said the engine
            // could not run one at all, and stayed there after it could.
            Topology::Fullscreen => {
                "the whole frame, from an L4 with **no `vertex` block**. It must \
                 `consumes` nothing — there is no element to read from — and it gets `eye` \
                 and `ray` in `fragment`, which nothing else does"
            }
        };
        out.push_str(&format!("- `{}` — {note}\n", topology.name()));
    }

    // Between the two, because a blend is declared where a topology is not and
    // the contrast is the point: a model that has just read "the renderer's
    // topology is inferred" will assume the same of `blend` unless told.
    out.push_str(
        "\n# Blend modes\n\nDeclared by an L4's `blend`, and **declared rather than \
         inferred** — unlike the topology above. Nothing an L4 writes could imply one over \
         the other, because the two differ in how the results of identical assignments are \
         combined.\n\nThey read `color`'s alpha differently, which is the part that \
         changes how a procedure is written.\n\n",
    );
    for blend in [Blend::Additive, Blend::Weighted] {
        let note = match blend {
            Blend::Additive => {
                "colour sums and nothing occludes. Alpha is **emission strength** and may \
                 exceed 1.0, scaling what the fragment adds"
            }
            // Named with the refusal it can meet, so that a model writing a
            // marcher does not reach for it and get a Set-build error it had no
            // way to predict.
            Blend::Weighted => {
                "order-independent transparency, so material **occludes** what is behind \
                 it. Alpha is **opacity** and is clamped to `[0, 1]`. Not available on a \
                 fullscreen L4: one fragment per texel makes it identical to `additive`, \
                 and building such a pair is refused"
            }
        };
        out.push_str(&format!("- `{}` — {note}\n", blend.name()));
    }

    out.push_str(
        "\n# Stage outputs\n\nAssigned like attributes; reading one is an error. \
                  `clip` and `point_size` are required in a `vertex` block, and `color` in a \
                  `fragment` block, on every path through it — but **a `vertex` block is \
                  itself optional**, which is how an L4 says it draws the whole frame. \
                  `clip_b` is the one optional output, and assigning it on only some paths \
                  is rejected.\n\n",
    );
    out.push_str("| name | type | block |\n|---|---|---|\n");
    for output in Output::ALL {
        out.push_str(&format!(
            "| `{}` | {} | `{}` |\n",
            output.name(),
            output.ty().name(),
            output.block().name()
        ));
    }
    out
}

#[cfg(test)]
mod wire_tests {
    //! **Over a socket, because everything else here passed with the server
    //! deleted.** A review mutated this module twelve ways — `handle` returning
    //! immediately, `serve` never binding, `respond` writing nothing,
    //! `read_procedure` returning a constant, `tools()` returning `[]` — and the
    //! suite was green for all twelve. A socket server whose tests never open a
    //! socket is not tested.

    use super::*;
    use std::io::{BufRead, BufReader, Write};

    struct Server {
        port: u16,
        dir: tempfile::TempDir,
    }

    /// **Under the fixture's own temporary directory and beside the procedure
    /// files, which is where a real run's is not.** A run's store is
    /// `--store DIR` and its procedures are wherever the operator keeps them;
    /// what matters to these tests is that the server and the test reach one
    /// root, and that a test that never writes a set leaves an empty store
    /// rather than reading one somebody else's run left behind.
    fn store_root(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().join("store")
    }

    impl Server {
        /// The library this server was started on, open from the test's side.
        fn store(&self) -> Store {
            Store::open(store_root(&self.dir)).expect("store")
        }
    }

    /// **The pair these tests serve, written out rather than copied from
    /// `examples/`.**
    ///
    /// It was a copy, and the examples are the files this very surface exists
    /// to rewrite — so the day a model renamed `soft_points` to something else
    /// over MCP, a test of *reading a procedure* failed on the new name. The
    /// comment inside `a_procedure_can_be_read_and_rewritten_over_the_wire`
    /// already recorded that lesson about the *write* half and the *read* half
    /// went on depending on the same file anyway.
    ///
    /// Minimal on purpose: nothing here is about what a procedure can express,
    /// only that one goes over the wire intact and comes back.
    const PROBE_L1: &str = r#"
proc probe_l1 {
  kind     L1
  topology points
  capacity [1, 64] = 8

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

    const PROBE_L4: &str = r#"
proc probe_l4 {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 2.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;

    /// **The other three layers, and a second source.** A slot is not a pair:
    /// it can hold deformations between the geometry and the renderers, one
    /// camera, one `kind Field`, and more than one L1 — and every one of them
    /// was a file this surface could not name. Written out for the reason the
    /// pair above is, and minimal for the same one.
    const PROBE_L2: &str = r#"
proc probe_warp {
  kind L2

  consumes position

  deform {
    position = vec3(position.x, position.y * 1.5, position.z);
  }
}
"#;

    const PROBE_L3: &str = r#"
proc probe_camera {
  kind L3

  camera {
    eye    = vec3(0.0, 2.0, 9.0);
    target = vec3(0.0, 0.0, 0.0);
  }
}
"#;

    const PROBE_FIELD: &str = r#"
proc probe_blob {
  kind Field

  field {
    distance = sd_sphere(point, 1.0);
  }
}
"#;

    const PROBE_L1_B: &str = r#"
proc probe_source_b {
  kind     L1
  topology points
  capacity [1, 64] = 8

  emit position

  element {
    position = vec3(1.0, 0.0, 0.0);
  }
}
"#;

    /// **A geometry with something declared on it**, for the tests about
    /// cards.
    ///
    /// The pair above declares no `param` and no range worth reading, so a
    /// rendering that dropped every `param_decl`, or one that printed a `min`
    /// where a `max` was, would pass against it. Every number here is a
    /// different number, and none of them is a number anything else in this
    /// module writes.
    const PROBE_KNOBS: &str = r#"
proc probe_knobs {
  kind     L1
  topology points
  capacity [16, 4096] = 256

  param radius : float [0.5, 3.5] = 1.75

  emit position

  element {
    position = vec3(radius, 0.0, 0.0);
  }
}
"#;

    /// A slot holding one of everything, in a file order that is deliberately
    /// not the order the layers compose in.
    ///
    /// **The second L1 comes last and the renderer is in the middle**, because
    /// an index that came from a file's position rather than from its place
    /// within its own layer passes any fixture where the two agree.
    fn start_chain() -> Server {
        let dir = tempfile::tempdir().expect("tempdir");
        let write = |name: &str, source: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, source).expect("fixture");
            path
        };
        let head = write("l1.kir", PROBE_L1);
        let rest = vec![
            write("warp.kir", PROBE_L2),
            write("l4.kir", PROBE_L4),
            write("camera.kir", PROBE_L3),
            write("blob.kir", PROBE_FIELD),
            write("l1_b.kir", PROBE_L1_B),
        ];
        let reporter = serve(0, Slots(vec![(head, rest)]), store_root(&dir), true).expect("serve");
        let port = reporter.port();
        stand_in(reporter, no_loop);
        Server { port, dir }
    }

    fn start(watching: bool) -> Server {
        let (server, reporter) = started(watching);
        stand_in(reporter, no_loop);
        server
    }

    /// The same fixture with the loop's half handed back, for the tests that
    /// are about what the loop says.
    fn started(watching: bool) -> (Server, Reporter) {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        let l4 = dir.path().join("l4.kir");
        std::fs::write(&l1, PROBE_L1).expect("l1");
        std::fs::write(&l4, PROBE_L4).expect("l4");
        // Port 0: the operating system picks, and `serve` reports what it got —
        // which is also the fix for `--mcp 0` naming a port that is not the port.
        let reporter =
            serve(0, Slots(vec![(l1, vec![l4])]), store_root(&dir), watching).expect("serve");
        let port = reporter.port();
        (Server { port, dir }, reporter)
    }

    /// **A stand-in for the render loop**, holding the [`Reporter`] the real one
    /// holds and answering the saves the server sends it.
    ///
    /// It also replaces the `std::mem::forget` that used to keep the reporter
    /// alive, and says what that was: the loop is the other half of this
    /// surface, and a `forget` is a loop that is present and permanently asleep
    /// — which is now a thing a client can wait on rather than only a channel
    /// that stays open.
    ///
    /// **Nothing here saves anything.** What a save *is* belongs to
    /// `Live::save_set` and needs a window and a GPU; what these tests are about
    /// is that a request crosses with its arguments intact and that whatever the
    /// loop says comes back to the client unchanged. So each fixture decides
    /// what the loop says.
    ///
    /// The thread never ends, which is what keeps the reporter alive.
    fn stand_in(reporter: Reporter, answer: impl Fn(SaveRequest) + Send + 'static) {
        std::thread::spawn(move || loop {
            for request in reporter.saves() {
                answer(request);
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        });
    }

    /// What the stand-in says for the tests that are not about saving — and it
    /// is true of the fixture rather than a stub of a real answer, because a
    /// test that met this by accident should read as a fixture problem and not
    /// as a save that failed.
    fn no_loop(request: SaveRequest) {
        let SaveRequest { slot, reply, .. } = request;
        reply.settled(Err(format!(
            "slot {slot}: this fixture has no render loop behind it"
        )));
    }

    /// One request, one reply, over TCP exactly as a client would.
    fn post(port: u16, body: &str) -> (u16, String) {
        raw(
            port,
            &format!(
                "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            ),
        )
    }

    fn raw(port: u16, request: &str) -> (u16, String) {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .ok();
        stream.write_all(request.as_bytes()).expect("write");
        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        // **Named, because the interesting way for this to fail is a timeout.**
        // A connection held up by another one reaches the client's own read
        // timeout and comes out here, and `expect("status")` reported that as an
        // errno rather than as what it is.
        reader.read_line(&mut status_line).expect(
            "no status line: the server did not answer this connection within the \
                     client's read timeout",
        );
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);
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
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    fn call(port: u16, name: &str, args: Value) -> (bool, String) {
        let (_, body) = post(
            port,
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                    "params":{"name":name,"arguments":args}})
            .to_string(),
        );
        let reply: Value = serde_json::from_str(&body).expect("json");
        let result = &reply["result"];
        (
            result["isError"].as_bool().unwrap_or(true),
            result["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        )
    }

    /// The loop the whole surface exists for: read, write, and the file changes.
    #[test]
    fn a_procedure_can_be_read_and_rewritten_over_the_wire() {
        let server = start(true);
        let (failed, source) = call(
            server.port,
            "read_procedure",
            json!({"slot":0,"layer":"L4"}),
        );
        assert!(!failed, "{source}");
        assert!(
            source.contains("proc probe_l4"),
            "{}",
            &source[..80.min(source.len())]
        );

        // **Prepended rather than substituted.** This asserted a phrase out of
        // the example's own comment header once, and broke the day somebody
        // rewrote the example — the substitution found nothing, the "edit" was
        // identical to the source, and the failure read as "the write did not
        // reach the file". A test of *writing* must not depend on what the
        // fixture happens to say.
        let edited = format!("// Edited over the wire.\n{source}");
        let (failed, said) = call(
            server.port,
            "write_procedure",
            json!({"slot":0,"layer":"L4","source":edited}),
        );
        assert!(!failed, "{said}");
        let on_disk = std::fs::read_to_string(server.dir.path().join("l4.kir")).expect("read back");
        assert!(
            on_disk.contains("Edited over the wire."),
            "the write did not reach the file"
        );
    }

    /// **Every node a slot holds is reachable, at the address the rest of this
    /// program already spells it by.**
    ///
    /// This surface reached the L1 and the renderers and nothing else, so the
    /// material a model could neither see nor edit was exactly the material
    /// this language is most interesting about: the deformation between the
    /// two, the camera, the field the renderers evaluate, and a second
    /// simulation source. Each address is checked against the *name* the file
    /// declares rather than against its position, because a resolver that had
    /// them one place out would still hand back a procedure.
    #[test]
    fn every_node_of_a_slot_can_be_read_at_its_own_address() {
        let server = start_chain();
        for (layer, index, expected) in [
            ("L1", 0, "proc probe_l1"),
            ("L2", 0, "proc probe_warp"),
            ("L3", 0, "proc probe_camera"),
            ("Field", 0, "proc probe_blob"),
            ("L4", 0, "proc probe_l4"),
            // The second source, which is an L1 in the chain rather than the
            // head — so its index is 1 and the head keeps 0.
            ("L1", 1, "proc probe_source_b"),
        ] {
            let (failed, source) = call(
                server.port,
                "read_procedure",
                json!({"slot":0,"layer":layer,"index":index}),
            );
            assert!(!failed, "{layer}:{index} could not be read: {source}");
            assert!(
                source.contains(expected),
                "{layer}:{index} read back the wrong file, which said: {}",
                source.lines().find(|l| l.starts_with("proc")).unwrap_or("")
            );
        }
    }

    /// **A deformation, a camera and a field are written as themselves.**
    ///
    /// The layer a write was checked against was `L1` or, for everything else,
    /// `L4` — so a `kind L2` sent to a slot's L2 was refused for not being a
    /// renderer, which is a refusal about a mistake nobody made. Both halves
    /// are asserted here: the writes that must land, and the one that must not.
    #[test]
    fn a_deformation_a_camera_and_a_field_are_written_as_themselves() {
        let server = start_chain();
        for (layer, file, source) in [
            ("L2", "warp.kir", PROBE_L2),
            ("L3", "camera.kir", PROBE_L3),
            ("Field", "blob.kir", PROBE_FIELD),
        ] {
            let edited = format!("// Edited over the wire.\n{source}");
            let (failed, said) = call(
                server.port,
                "write_procedure",
                json!({"slot":0,"layer":layer,"source":edited}),
            );
            assert!(!failed, "a {layer} could not be written: {said}");
            let on_disk = std::fs::read_to_string(server.dir.path().join(file)).expect("read back");
            assert!(
                on_disk.contains("Edited over the wire."),
                "the {layer} write did not reach {file}"
            );
        }

        // And the refusal is still honest: the source has to declare the layer
        // it was addressed to, whichever layer that is.
        let (failed, said) = call(
            server.port,
            "write_procedure",
            json!({"slot":0,"layer":"L4","source":PROBE_L2}),
        );
        assert!(failed, "a deformation was written over a renderer");
        assert!(said.contains("not interchangeable"), "{said}");
    }

    /// **The schema offers every layer a slot can hold**, because a layer a
    /// client is not told about is one it will not ask for — the enum said
    /// `L1` and `L4` for as long as a slot could hold five kinds of node.
    #[test]
    fn the_advertised_layers_are_every_layer_a_slot_can_hold() {
        let server = start_chain();
        let (_, listed) = post(
            server.port,
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        );
        let listed: Value = serde_json::from_str(&listed).expect("json");
        let tools = listed["result"]["tools"].as_array().expect("tools");
        let expected = json!(["L1", "L2", "L3", "L4", "Field"]);
        for name in ["read_procedure", "write_procedure"] {
            let tool = tools
                .iter()
                .find(|t| t["name"] == json!(name))
                .unwrap_or_else(|| panic!("`{name}` is not advertised"));
            assert_eq!(
                tool["inputSchema"]["properties"]["layer"]["enum"], expected,
                "`{name}` offers a client the wrong layers"
            );
        }

        // And what is advertised is what answers: every advertised layer
        // resolves to something on a slot that holds one of each.
        for layer in expected.as_array().expect("layers") {
            let (failed, said) = call(
                server.port,
                "read_procedure",
                json!({"slot":0,"layer":layer}),
            );
            assert!(
                !failed,
                "`{layer}` is advertised and does not resolve: {said}"
            );
        }
    }

    /// A procedure that does not compile never reaches the disk, and what comes
    /// back is the checker's words — **the return value is the point**.
    #[test]
    fn a_procedure_that_does_not_compile_is_refused_with_diagnostics() {
        let server = start(true);
        let before = std::fs::read_to_string(server.dir.path().join("l4.kir")).expect("before");
        let (failed, said) = call(
            server.port,
            "write_procedure",
            json!({"slot":0,"layer":"L4","source":"proc broken {\n  out color = nope(1.0)\n}\n"}),
        );
        assert!(failed, "a broken procedure was accepted: {said}");
        assert!(said.contains("parse"), "{said}");
        assert_eq!(
            std::fs::read_to_string(server.dir.path().join("l4.kir")).expect("after"),
            before,
            "a procedure that does not compile reached the disk"
        );
    }

    /// An L1 procedure is not an L4 one, and the slot says which it wanted.
    #[test]
    fn a_procedure_for_the_other_layer_is_refused() {
        let server = start(true);
        let (_, l1) = call(
            server.port,
            "read_procedure",
            json!({"slot":0,"layer":"L1"}),
        );
        let (failed, said) = call(
            server.port,
            "write_procedure",
            json!({"slot":0,"layer":"L4","source":l1}),
        );
        assert!(failed, "an L1 procedure was written into L4");
        assert!(said.contains("not interchangeable"), "{said}");
    }

    /// **A body larger than this server will read is refused before it is
    /// allocated.** `vec![0u8; length]` on an attacker's number aborts the
    /// process — not a panic, not catchable, and not confined to this thread.
    /// A fifty-six byte request line used to take the render process down.
    #[test]
    fn an_enormous_content_length_is_refused_and_not_allocated() {
        let server = start(true);
        let (status, _) = raw(
            server.port,
            "POST / HTTP/1.1\r\nContent-Length: 1152921504606846976\r\n\r\n",
        );
        assert_eq!(status, 413, "an exabyte body was not refused");
        // And the server is still there afterwards, which is the whole claim.
        let (status, _) = post(
            server.port,
            &json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string(),
        );
        assert_eq!(status, 200, "the server did not survive");
    }

    /// **A page on any site can POST here.** It cannot read the reply, and
    /// `write_procedure` does not need to be read to have happened. The first
    /// version of this server treated loopback as a boundary; it is not one.
    #[test]
    fn a_cross_origin_request_is_refused() {
        let server = start(true);
        let body = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":"write_procedure",
            "arguments":{"slot":0,"layer":"L4","source":"proc x {\n}\n"}}})
        .to_string();
        let (status, _) = raw(
            server.port,
            &format!(
                "POST / HTTP/1.1\r\nOrigin: https://evil.example\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            ),
        );
        assert_eq!(status, 403, "a cross-origin write was answered");
        // A local client sends no Origin at all and is still served.
        let (status, _) = post(
            server.port,
            &json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string(),
        );
        assert_eq!(status, 200);
    }

    /// The body of a non-POST used to be left in the reader and become the next
    /// request line, so a `GET` with a body ran a smuggled call.
    /// **A path this server does not have is a 404, and that is what lets a
    /// client connect at all.**
    ///
    /// A client's first move is authorization discovery:
    /// `GET /.well-known/oauth-protected-resource`. This server answered 405 to
    /// every path, which says "that resource exists, just not by this verb" —
    /// so the client went off to fetch protected-resource metadata, tried to
    /// parse `this server only answers POST` as JSON, and reported the server
    /// as unreachable. Nothing was unreachable; the handshake died on a path
    /// that has never existed here.
    ///
    /// Asserted over a socket rather than against a handler, because a status
    /// code is a property of the wire — see this module's other wire tests for
    /// why that distinction has already mattered here.
    #[test]
    fn a_path_this_server_does_not_serve_is_not_found_rather_than_not_allowed() {
        let server = start(true);
        for path in [
            "/.well-known/oauth-protected-resource",
            "/.well-known/oauth-authorization-server",
            "/mcp",
        ] {
            let (status, body) = raw(
                server.port,
                &format!("GET {path} HTTP/1.1\r\nContent-Length: 0\r\n\r\n"),
            );
            assert_eq!(status, 404, "GET {path} answered {status}");
            // And a body a JSON client can read, because the one that got here
            // was parsing JSON when it failed.
            serde_json::from_str::<Value>(&body)
                .unwrap_or_else(|e| panic!("the 404 body for {path} is not JSON: {e} — {body}"));
        }
    }

    /// The endpoint itself still answers 405 to a GET, which is the Streamable
    /// HTTP transport's own rule for a server offering no SSE stream there.
    ///
    /// The control for the test above: answering 404 everywhere would satisfy
    /// it and break the transport.
    #[test]
    fn the_endpoint_itself_answers_405_to_a_get() {
        let server = start(true);
        let (status, body) = raw(server.port, "GET / HTTP/1.1\r\nContent-Length: 0\r\n\r\n");
        assert_eq!(status, 405);
        serde_json::from_str::<Value>(&body).expect("the 405 body is JSON too");
    }

    /// And a POST to a path that is not the endpoint is a 404 as well — the
    /// path decides, not the verb.
    #[test]
    fn a_post_to_another_path_is_also_not_found() {
        let server = start(true);
        let body = json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string();
        let (status, _) = raw(
            server.port,
            &format!(
                "POST /somewhere HTTP/1.1\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            ),
        );
        assert_eq!(status, 404);
    }

    #[test]
    fn a_non_post_cannot_smuggle_a_second_request() {
        let server = start(true);
        let smuggled = json!({"jsonrpc":"2.0","id":99,"method":"ping"}).to_string();
        let (status, body) = raw(
            server.port,
            &format!(
                "GET / HTTP/1.1\r\nContent-Length: {}\r\n\r\nPOST / HTTP/1.1\r\nContent-Length: {}\r\n\r\n{smuggled}",
                smuggled.len() + 60,
                smuggled.len()
            ),
        );
        assert_eq!(status, 405);
        assert!(
            !body.contains("\"id\":99"),
            "the smuggled request ran: {body}"
        );
    }

    /// Field names are case-insensitive, and a length that is not a number is
    /// said rather than read as zero — which used to tell the client its JSON
    /// was malformed when it was not.
    #[test]
    fn header_names_are_case_insensitive_and_a_bad_length_is_named() {
        let server = start(true);
        let body = json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string();
        let (status, reply) = raw(
            server.port,
            &format!(
                "POST / HTTP/1.1\r\nCONTENT-LENGTH: {}\r\n\r\n{body}",
                body.len()
            ),
        );
        assert_eq!(status, 200, "an uppercase header name was not understood");
        assert!(reply.contains("result"), "{reply}");

        let (status, said) = raw(
            server.port,
            "POST / HTTP/1.1\r\nContent-Length: 12x\r\n\r\n",
        );
        assert_eq!(status, 400);
        assert!(said.contains("not a number"), "{said}");
    }

    /// One connection that says nothing must not take the surface with it.
    #[test]
    fn a_silent_connection_does_not_wedge_the_server() {
        let server = start(true);
        let _silent = std::net::TcpStream::connect(("127.0.0.1", server.port)).expect("connect");
        // The first version served every connection on one thread, so this
        // second one waited for the first to hang up — which it never does.
        let (status, _) = post(
            server.port,
            &json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string(),
        );
        assert_eq!(status, 200, "a silent socket wedged the server");
    }

    /// A write also reaches every slot sharing that file, and says so. The
    /// manual's own example gives one `soft_points.kir` to three slots.
    #[test]
    fn a_write_names_the_other_slots_it_reached() {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        let l4 = dir.path().join("l4.kir");
        std::fs::write(&l1, PROBE_L1).expect("l1");
        std::fs::write(&l4, PROBE_L4).expect("l4");
        let shared = Slots(vec![(l1.clone(), vec![l4.clone()]), (l1, vec![l4])]);
        let reporter = serve(0, shared, store_root(&dir), true).expect("serve");
        let port = reporter.port();
        std::mem::forget(reporter);

        let (_, source) = call(port, "read_procedure", json!({"slot":0,"layer":"L4"}));
        let (failed, said) = call(
            port,
            "write_procedure",
            json!({"slot":0,"layer":"L4","source":source}),
        );
        assert!(!failed, "{said}");
        assert!(
            said.contains("also slot 1"),
            "the other slot was not named: {said}"
        );
        // **Where the version it replaced went.** This said "there is no
        // backup" while `--watch` was snapshotting every version that compiled
        // into the edit history — and it is the text a model reads, so the one
        // surface that could have told it the file was recoverable said the
        // opposite.
        assert!(
            said.contains("history"),
            "where the old version went: {said}"
        );
    }

    /// **A file shared as anything but a renderer is shared exactly as much.**
    ///
    /// The scan behind that sentence walked an L1 and a list of renderers,
    /// which is the shape a slot had before it could hold a deformation chain
    /// — so one `swirl_warp.kir` given to two slots was a write that changed
    /// both and named one, and the count a model is handed is only worth
    /// having if it is the whole count.
    #[test]
    fn a_write_names_the_other_slots_it_reached_on_any_layer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let write = |name: &str, source: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, source).expect("fixture");
            path
        };
        let l1 = write("l1.kir", PROBE_L1);
        let warp = write("warp.kir", PROBE_L2);
        let l4 = write("l4.kir", PROBE_L4);
        let shared = Slots(vec![
            (l1.clone(), vec![warp.clone(), l4.clone()]),
            (l1, vec![warp, l4]),
        ]);
        let reporter = serve(0, shared, store_root(&dir), true).expect("serve");
        let port = reporter.port();
        std::mem::forget(reporter);

        let (failed, said) = call(
            port,
            "write_procedure",
            json!({"slot":0,"layer":"L2","source":PROBE_L2}),
        );
        assert!(!failed, "{said}");
        assert!(
            said.contains("also slot 1"),
            "the slot sharing this deformation was not named: {said}"
        );
    }

    /// The tools and resources a client is offered are the ones that answer.
    #[test]
    fn everything_advertised_can_be_called() {
        let server = start(true);
        let (_, listed) = post(
            server.port,
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        );
        let listed: Value = serde_json::from_str(&listed).expect("json");
        let names: Vec<String> = listed["result"]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .map(|t| t["name"].as_str().expect("name").to_string())
            .collect();
        assert!(!names.is_empty(), "no tools were advertised");
        for name in &names {
            let (_, said) = call(server.port, name, json!({"slot":0,"layer":"L4"}));
            assert!(
                !said.is_empty(),
                "`{name}` is advertised and answers nothing"
            );
        }

        let (_, listed) = post(
            server.port,
            &json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}).to_string(),
        );
        let listed: Value = serde_json::from_str(&listed).expect("json");
        let uris: Vec<String> = listed["result"]["resources"]
            .as_array()
            .expect("resources")
            .iter()
            .map(|r| r["uri"].as_str().expect("uri").to_string())
            .collect();
        assert!(!uris.is_empty(), "no resources were advertised");
        for uri in &uris {
            let (_, body) = post(
                server.port,
                &json!({"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":uri}})
                    .to_string(),
            );
            let reply: Value = serde_json::from_str(&body).expect("json");
            let text = reply["result"]["contents"][0]["text"]
                .as_str()
                .unwrap_or("");
            assert!(
                text.len() > 100,
                "`{uri}` is advertised and reads as nothing"
            );
        }
    }

    /// **The save tool is offered, and a call reaches the loop with what it was
    /// given and comes back with what the loop said.**
    ///
    /// The whole of the new channel in one pass: advertised, sent, answered.
    /// The stand-in asserts the arguments it was handed, because a request that
    /// arrived with the wrong slot or no id would still have produced an answer.
    #[test]
    fn the_save_tool_is_offered_and_a_call_reaches_the_loop() {
        // **Two slots**, so that the slot the loop is handed is a fact about
        // the call rather than the only slot there is: a request that carried a
        // constant would pass against a one-slot deck.
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        let l4 = dir.path().join("l4.kir");
        std::fs::write(&l1, PROBE_L1).expect("l1");
        std::fs::write(&l4, PROBE_L4).expect("l4");
        let pair = (l1, vec![l4]);
        let reporter =
            serve(0, Slots(vec![pair.clone(), pair]), store_root(&dir), true).expect("serve");
        let server = Server {
            port: reporter.port(),
            dir,
        };
        stand_in(reporter, |request| {
            let SaveRequest { slot, id, reply } = request;
            assert_eq!(slot, 1, "the request reached the loop naming another slot");
            let id = id.expect("the id the client named did not reach the loop");
            reply.accepted(&format!(
                "slot {slot}: saving 2 nodes as set `{id}` in <store>"
            ));
            reply.settled(Ok(format!(
                "slot {slot}: saved as set `{id}` — load it with `--load-set {id}`"
            )));
        });

        let (_, listed) = post(
            server.port,
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        );
        let listed: Value = serde_json::from_str(&listed).expect("json");
        let names: Vec<&str> = listed["result"]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(
            names.contains(&"save_set"),
            "a model cannot call what it is not offered: {names:?}"
        );

        let (failed, said) = call(server.port, "save_set", json!({"slot":1,"id":"keeper"}));
        assert!(!failed, "{said}");
        assert!(
            said.contains("--load-set keeper"),
            "the call came back without the id it was saved under: {said}"
        );
    }

    /// **A slot that cannot be saved comes back with the loop's own refusal**,
    /// unchanged.
    ///
    /// The sentence comes from [`crate::nothing_to_save`] rather than being
    /// written out here: a copy would go on passing after the real refusal was
    /// corrected, which is exactly what happened to this wording once already —
    /// it named `--watch` alone for as long as `--mcp` also made a slot savable.
    #[test]
    fn a_slot_that_cannot_be_saved_comes_back_with_the_loops_own_words() {
        let (server, reporter) = started(true);
        stand_in(reporter, |request| {
            let SaveRequest { slot, reply, .. } = request;
            reply.settled(Err(crate::nothing_to_save(slot, Some("night01"), true)));
        });
        let (failed, said) = call(server.port, "save_set", json!({"slot":0}));
        assert!(failed, "a refusal came back as a success: {said}");
        assert_eq!(
            said,
            crate::nothing_to_save(0, Some("night01"), true),
            "the refusal was rewritten on its way to the client"
        );
    }

    /// **A slot this deck does not hold meets the refusal `read_procedure`
    /// gives it — which is the refusal a key press meets**, without troubling
    /// the render loop at all.
    ///
    /// `contains("no slot 7")` was all this asked, and it passed under all four
    /// spellings this program had of one sentence: the keys' `no slot 7: this
    /// deck holds slots 0-6`, this module's `holds 0-6`, MIDI's `no slot 7 —
    /// …`, and a record's `slot 7: …`. It is `assert_eq!` against
    /// [`crate::no_such_slot`] now, on both surfaces of this module, because
    /// `save_set` is the control a model and a hand both reach and the wording
    /// they get for one mistake has to be one wording. See that function.
    #[test]
    fn a_save_for_a_slot_that_does_not_exist_is_refused_here() {
        let (server, reporter) = started(true);
        stand_in(reporter, |request| {
            let SaveRequest { slot, reply, .. } = request;
            reply.settled(Err(format!(
                "slot {slot}: the loop was asked about a slot this deck does not hold"
            )));
        });
        let (failed, said) = call(server.port, "save_set", json!({"slot":7}));
        assert!(failed, "{said}");
        assert_eq!(
            said,
            crate::no_such_slot(7, 1),
            "a bad slot was not refused in the words every other surface refuses it in"
        );
        assert!(
            !said.contains("the loop"),
            "a slot this deck does not hold was sent to the render loop: {said}"
        );

        // The other door to the same refusal, which is where this module's own
        // spelling used to live.
        let (failed, said) = call(
            server.port,
            "read_procedure",
            json!({"slot":7,"layer":"L1"}),
        );
        assert!(failed, "{said}");
        assert_eq!(said, crate::no_such_slot(7, 1));
    }

    /// **`"id": null` is a caller saying nothing about the id**, not a caller
    /// getting its type wrong.
    ///
    /// A client that builds its arguments from a record with an empty field
    /// sends `null` for an argument it is not using, and this refused it with
    /// "`id` is a string" — a refusal about a mistake the caller had not made,
    /// and one it cannot act on, since what it wanted was the default. Every
    /// other optional argument on this surface reads an absent one as its
    /// default; `null` is absent's second spelling.
    #[test]
    fn a_null_id_is_an_absent_id_and_not_a_bad_one() {
        let (server, reporter) = started(true);
        stand_in(reporter, |request| {
            let SaveRequest { slot, id, reply } = request;
            // What the loop makes of `None` is a stamp; what this test is about
            // is that it was handed `None` rather than the call being refused
            // before it got there.
            reply.settled(Ok(match id {
                None => format!("slot {slot}: the loop was left to name it"),
                Some(id) => format!("slot {slot}: the loop was handed `{id}`"),
            }));
        });

        let (failed, said) = call(server.port, "save_set", json!({"slot":0,"id":null}));
        assert!(!failed, "{said}");
        assert!(
            said.contains("left to name it"),
            "`null` was refused as a bad string rather than read as an absent id: {said}"
        );

        // And an argument that really is the wrong type still is one.
        let (failed, said) = call(server.port, "save_set", json!({"slot":0,"id":7}));
        assert!(failed, "{said}");
        assert!(said.contains("`id` is a string"), "{said}");
    }

    /// **A save in flight does not hold up another connection**, which is the
    /// evidence rather than a comment saying the lock was dropped.
    ///
    /// `handle` locks one mutex around `dispatch` and runs a thread per
    /// connection, so a tool that waited for the render loop under that lock
    /// would stop every other client for as long as the loop took. Here the
    /// stand-in has taken a save and is holding it; a second connection asks for
    /// a procedure and must be answered before the first is released.
    ///
    /// The second call is not merely *started* while the first is in flight —
    /// it is started only once the stand-in has the request in its hands, so
    /// there is no ordering in which this passes by racing ahead of the wait.
    #[test]
    fn a_save_in_flight_does_not_block_another_connection() {
        let (server, reporter) = started(true);
        let (entered, arrived) = mpsc::channel::<()>();
        let (release, released) = mpsc::channel::<()>();
        let released = std::sync::Mutex::new(released);
        stand_in(reporter, move |request| {
            let SaveRequest { slot, id, reply } = request;
            let id = id.unwrap_or_else(|| "stamped".to_string());
            reply.accepted(&format!(
                "slot {slot}: saving 2 nodes as set `{id}` in <store>"
            ));
            entered.send(()).expect("the test is listening");
            released.lock().expect("release").recv().expect("released");
            reply.settled(Ok(format!(
                "slot {slot}: saved as set `{id}` — load it with `--load-set {id}`"
            )));
        });

        let port = server.port;
        let waiting =
            std::thread::spawn(move || call(port, "save_set", json!({"slot":0,"id":"slow"})));
        arrived
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("the save never reached the stand-in loop");

        // The evidence. Under a held lock this blocks until the client's own
        // read timeout gives up on it.
        let (failed, source) = call(
            server.port,
            "read_procedure",
            json!({"slot":0,"layer":"L4"}),
        );
        assert!(!failed, "a second connection was refused: {source}");
        assert!(source.contains("proc probe_l4"), "{source}");

        release.send(()).expect("release the save");
        let (failed, said) = waiting.join().expect("the waiting call");
        assert!(!failed, "{said}");
        assert!(said.contains("--load-set slow"), "{said}");
    }

    /// `initialize` answers with what a client needs to proceed.
    #[test]
    fn initialize_answers_with_a_protocol_version_and_capabilities() {
        let server = start(true);
        let (status, body) = post(
            server.port,
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize"}).to_string(),
        );
        assert_eq!(status, 200);
        let reply: Value = serde_json::from_str(&body).expect("json");
        assert_eq!(reply["result"]["protocolVersion"], json!(PROTOCOL));
        assert!(reply["result"]["capabilities"]["tools"].is_object());
        assert_eq!(reply["result"]["serverInfo"]["name"], json!("karakuri"));
    }

    /// Without `--watch` a write changes a file and nothing else, and the model
    /// is told so — it has no other way to find out.
    #[test]
    fn a_write_without_watch_says_nothing_will_pick_it_up() {
        let server = start(false);
        let (_, source) = call(
            server.port,
            "read_procedure",
            json!({"slot":0,"layer":"L4"}),
        );
        let (failed, said) = call(
            server.port,
            "write_procedure",
            json!({"slot":0,"layer":"L4","source":source}),
        );
        assert!(!failed, "{said}");
        assert!(said.contains("--watch"), "{said}");
    }

    /// **A stored artifact, its card, and one Set naming it** — the fixture the
    /// card tests share.
    ///
    /// It puts the source and writes the card through [`crate::meta::card`]
    /// rather than by hand, because what these tests are about is that the
    /// numbers a model reads are the numbers the *source* declared: a card
    /// assembled in the test would only prove this module can render a record
    /// it was handed.
    fn kept(server: &Server, id: &str, source: &str, card: bool) -> Hash {
        let store = server.store();
        let hash = store.put_artifact(source.as_bytes()).expect("put");
        if card {
            let checked = crate::compile::check(source).expect("the fixture compiles");
            store
                .write_meta(&hash, &crate::meta::card(&hash, &checked))
                .expect("card");
        }
        set_naming(server, id, hash);
        hash
    }

    /// A Set file naming one node, plus a record that is not a `slot`.
    ///
    /// The `param` is there so the reader has something to pass over: it is a
    /// value this Set holds, which is a different question from what the
    /// artifact declares, and a reader folding the two together would render it
    /// as a knob.
    fn set_naming(server: &Server, id: &str, hash: Hash) {
        server
            .store()
            .write_set(
                id,
                &[
                    Line::new(Record::Slot {
                        layer: Layer::L1,
                        index: 0,
                        name: Some("shell".into()),
                        proc_hash: hash,
                    }),
                    Line::new(Record::Param {
                        layer: Layer::L1,
                        index: Some(0),
                        key: "radius".into(),
                        value: karakuri_store::record::Value::Scalar(2.5),
                    }),
                ],
            )
            .expect("set");
    }

    /// **The tool is offered, and what comes back is what the source
    /// declared.**
    ///
    /// The two halves are one test on purpose: a tool that is advertised and
    /// answers nothing, and one that answers without being advertised, are both
    /// invisible to a client, and this is the pass that says a model can find it
    /// and use it in one go.
    #[test]
    fn the_set_tool_is_offered_and_a_card_says_what_the_source_declared() {
        let server = start(true);
        kept(&server, "keeper", PROBE_KNOBS, true);

        let (_, listed) = post(
            server.port,
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        );
        let listed: Value = serde_json::from_str(&listed).expect("json");
        let names: Vec<String> = listed["result"]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .map(|t| t["name"].as_str().expect("name").to_string())
            .collect();
        assert!(
            names.iter().any(|name| name == "read_set"),
            "a client is never told the tool exists: {names:?}"
        );

        let (failed, said) = call(server.port, "read_set", json!({"id":"keeper"}));
        assert!(!failed, "{said}");
        // **The declaration, number for number.** Each of these is in the
        // `.kir` above and in no other fixture, so a rendering that reached for
        // the wrong end of a range, or that answered off a card it built itself,
        // says a number that is not here.
        for expected in [
            "probe_knobs",
            "L1:0",
            "`shell`",
            "param radius : float, anywhere from 0.5 to 3.5",
            "and 1.75 until something turns it",
            "between 16 and 4096 elements, and 256 of them",
            "emits position",
        ] {
            assert!(
                said.contains(expected),
                "the card does not say `{expected}`: {said}"
            );
        }
        // **What the Set turned it to is not what the artifact declares.** The
        // fixture's `param` record holds 2.5 and the tool answers about
        // declarations; a reader folding the two would print it as a range or as
        // a default, and either is the `param_decl` / `param` confusion the
        // record vocabulary keeps two names to prevent.
        assert!(
            !said.contains("2.5"),
            "a value this set holds was rendered as something the artifact \
             declares: {said}"
        );
    }

    /// **An artifact with no card is described, not reported as a broken
    /// store.**
    ///
    /// `Store::put_artifact` writes no card of its own — it takes bytes and does
    /// not compile — so this is the ordinary state of anything stored before
    /// cards existed or stored without one, and `Store::read_meta` answers it
    /// with the same `NotFound` it answers a damaged library with. What a model
    /// must not be handed is a failed call about a store that is fine.
    #[test]
    fn an_artifact_with_no_card_is_answered_and_not_called_a_broken_store() {
        let server = start(true);
        kept(&server, "uncarded", PROBE_KNOBS, false);
        let (failed, said) = call(server.port, "read_set", json!({"id":"uncarded"}));
        assert!(
            !failed,
            "an artifact stored without a card was reported to a model as a failed \
             call: {said}"
        );
        assert!(
            said.contains("L1:0") && said.contains("no metadata card"),
            "the node was not described at all: {said}"
        );
        assert!(
            said.contains("not a damaged store"),
            "a card nobody has written yet reads as damage: {said}"
        );

        // **A hash this store has never seen is the other absence**, and it is a
        // different fact: the Set cannot be loaded here at all. Both arrive as
        // one `NotFound`, so a reader that did not ask the second question tells
        // a model to go read a source that is not there.
        set_naming(&server, "elsewhere", Hash::of(b"stored on another machine"));
        let (failed, said) = call(server.port, "read_set", json!({"id":"elsewhere"}));
        assert!(!failed, "{said}");
        assert!(
            said.contains("does not hold that artifact"),
            "a set naming material this store has never had was answered as though \
             the source were here: {said}"
        );
    }

    /// **A set id from a client is one path component on the way to a card as
    /// much as on the way to a save.**
    ///
    /// `save_set` puts a client's id through [`checked_id`] and this reads a
    /// file under `<store>/sets/` by the same spelling — paths never cross this
    /// protocol, and a *read* is the direction that hands the file back.
    #[test]
    fn a_set_id_on_the_way_to_a_card_cannot_name_a_path() {
        let server = start(true);
        for bad in [
            "../../../etc/passwd",
            "sets/../../elsewhere",
            "a/b",
            "~/mine",
        ] {
            let (failed, said) = call(server.port, "read_set", json!({"id": bad}));
            assert!(failed, "`{bad}` was accepted as a set id: {said}");
            // **Refused before anything was opened.** The refusal names the rule
            // rather than an errno, which is also how it is told apart from the
            // one a real read of a missing file produces.
            assert!(
                said.contains("path component") || said.contains("letters, digits"),
                "`{bad}` was refused for something other than being a path: {said}"
            );
            assert!(
                !said.contains("reading set"),
                "`{bad}` reached the filesystem: {said}"
            );
        }
    }

    /// A set nobody saved is refused by the id that was asked for, and says
    /// where sets come from — the answer a model can act on, against an errno
    /// it cannot.
    #[test]
    fn a_set_this_store_never_saw_is_refused_by_its_id() {
        let server = start(true);
        let (failed, said) = call(server.port, "read_set", json!({"id":"never_saved"}));
        assert!(
            failed,
            "a set that is not there answered as though it were: {said}"
        );
        assert!(said.contains("never_saved"), "{said}");
        assert!(said.contains("save_set"), "{said}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state with the loop's half of both channels missing, for the tests
    /// that are about what one method answers rather than about a render loop.
    ///
    /// The save channel's receiver is dropped on the way out, which is exactly
    /// the "the loop is gone" case: anything that tried to ask for a save here
    /// would be told so rather than wait.
    fn state(events: mpsc::Receiver<Event>) -> State {
        State {
            slots: slots(),
            // **A root, and nothing here opens it.** Only `read_set` does, on
            // the call, which is what lets every test in this module build a
            // state without a directory — see `serve`.
            store: "a/store".into(),
            watching: true,
            events,
            asked: mpsc::sync_channel(ASKED).0,
            recent: Vec::new(),
            dropped: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Two slots of a head and one more file, and **none of these paths
    /// exists**.
    ///
    /// That is deliberate rather than lazy. A layer is read off a file's own
    /// `kind` line, and a file that cannot be read counts as a renderer — the
    /// fallback [`Slots::nodes`] shares with `history::seed`, asserted in
    /// [`an_unreadable_file_is_counted_as_a_renderer`] and relied on here, so
    /// these two slots are the L1-and-one-renderer pair they read as.
    fn slots() -> Slots {
        Slots(vec![
            ("a/l1.kir".into(), vec!["a/l4.kir".into()]),
            ("b/l1.kir".into(), vec!["b/l4.kir".into()]),
        ])
    }

    /// Writes `name` declaring `kind`, and **nothing that would compile**.
    ///
    /// A layer is scanned out of the text rather than parsed, so that this
    /// surface works on a file the checker would refuse — which is the file a
    /// model most needs to be able to read. A fixture that compiled would not
    /// say so.
    fn declaring(dir: &std::path::Path, name: &str, kind: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, format!("kind {kind}\nnot a procedure at all\n")).expect("fixture");
        path
    }

    /// The text under one `# ` heading of the rendered vocabulary.
    ///
    /// The two halves of the test below ask opposite questions of one section
    /// each, and mixing sections silently weakens both — the "nothing
    /// invented" half went looking for `clip` in `Builtin::from_name` the
    /// moment stage outputs were added to the page, which is the failure
    /// working rather than a nuisance.
    fn section<'a>(rendered: &'a str, heading: &str) -> &'a str {
        let start = rendered
            .find(heading)
            .unwrap_or_else(|| panic!("the vocabulary has no `{heading}` section"));
        let rest = &rendered[start + heading.len()..];
        match rest.find("\n# ") {
            Some(end) => &rest[..end],
            None => rest,
        }
    }

    /// **The vocabulary is generated, so it cannot say a function exists that
    /// does not.** That is the whole reason it is served beside the prose spec:
    /// `docs/ir-spec.md` describes this language in English and English goes
    /// stale, where this list is the one the checker matches against.
    #[test]
    fn the_vocabulary_is_the_checkers_own_table() {
        let rendered = vocabulary();
        let builtins = section(&rendered, "# Built-in functions");
        for builtin in karakuri_ir::builtin::Builtin::ALL {
            assert!(
                builtins.contains(&format!("| `{}` |", builtin.name())),
                "`{}` is accepted by the checker and missing from the vocabulary",
                builtin.name()
            );
        }
        // And nothing invented: every row names something `from_name` knows.
        for line in builtins.lines().filter(|l| l.starts_with("| `")) {
            let name = line
                .trim_start_matches("| `")
                .split('`')
                .next()
                .expect("a name");
            assert!(
                karakuri_ir::builtin::Builtin::from_name(name).is_some(),
                "the vocabulary lists `{name}`, which the checker does not know"
            );
        }
    }

    /// The same claim about the two other closed vocabularies a procedure is
    /// written against — the topologies and the stage outputs.
    ///
    /// A model that is told the wrong set here writes a file the checker
    /// refuses, which is the cheap failure; one that is told *too few* never
    /// discovers a whole rendering mode, which is not cheap at all. `clip_b`
    /// is the case in point: it is the only way to draw a segment, and a page
    /// that omitted it would leave the language looking exactly as it did
    /// before lines existed.
    #[test]
    fn the_vocabulary_lists_every_topology_every_blend_and_every_stage_output() {
        let rendered = vocabulary();

        let topologies = section(&rendered, "# Topologies");
        for name in ["points", "lines"] {
            assert!(
                topologies.contains(&format!("`{name}`")),
                "the vocabulary does not mention the `{name}` topology"
            );
            assert!(
                karakuri_ir::parse(&format!(
                    "proc p {{ kind L1 topology {name} capacity [1, 2] = 1 \
                     emit position element {{ position = vec3(0.0, 0.0, 0.0); }} }}"
                ))
                .is_ok(),
                "the vocabulary lists `{name}`, which the parser does not accept"
            );
        }

        // **Every blend mode, and each one round-tripped through the parser**,
        // for the same reason the topologies are: a page listing a mode the
        // language does not accept sends a model into a diagnostic, and one
        // omitting a mode hides a whole way of drawing. `weighted` is this
        // milestone's `clip_b` — the only way to make material occlude
        // anything, and invisible to anyone not told it exists.
        let blends = section(&rendered, "# Blend modes");
        for name in ["additive", "weighted"] {
            assert!(
                blends.contains(&format!("`{name}`")),
                "the vocabulary does not mention the `{name}` blend mode"
            );
            assert!(
                karakuri_ir::parse(&format!(
                    "proc p {{ kind L4 blend {name} consumes position \
                     vertex {{ clip = vec4(position, 1.0); point_size = 1.0; }} \
                     fragment {{ color = vec4(1.0, 1.0, 1.0, 1.0); }} }}"
                ))
                .is_ok(),
                "the vocabulary lists `{name}`, which the parser does not accept"
            );
        }

        let outputs = section(&rendered, "# Stage outputs");
        for output in karakuri_ir::Output::ALL {
            assert!(
                outputs.contains(&format!("| `{}` |", output.name())),
                "`{}` is assignable and missing from the vocabulary",
                output.name()
            );
        }
        for line in outputs.lines().filter(|l| l.starts_with("| `")) {
            let name = line
                .trim_start_matches("| `")
                .split('`')
                .next()
                .expect("a name");
            assert!(
                karakuri_ir::Output::from_name(name).is_some(),
                "the vocabulary lists an output `{name}` the checker does not know"
            );
        }
    }

    /// A slot is a number and a layer is one of five. **No path crosses the
    /// protocol**: a client may be on another machine through an `ssh -L`,
    /// where a path means nothing — and a tool that took one would invite a
    /// model to write anywhere on the render machine's disk.
    #[test]
    fn a_slot_a_layer_and_a_renderer_resolve_and_anything_else_is_refused() {
        let slots = slots();
        assert_eq!(
            slots.path(1, Kind::L4, 0).expect("slot 1 L4"),
            &std::path::PathBuf::from("b/l4.kir")
        );
        assert_eq!(
            slots.path(0, Kind::L1, 0).expect("slot 0 L1"),
            &std::path::PathBuf::from("a/l1.kir")
        );

        let past_the_end = slots
            .path(2, Kind::L1, 0)
            .expect_err("slot 2 does not exist");
        assert_eq!(past_the_end, crate::no_such_slot(2, 2));
        // And the cheap door to the same answer, which is what `save_set` asks
        // rather than reading every file of a slot to learn a length.
        assert_eq!(
            slots.holds(2).expect_err("slot 2 does not exist"),
            past_the_end
        );
        assert!(slots.holds(1).is_ok());

        // **A layer this slot does not use is a different answer from a layer
        // this surface cannot reach**, and it used to give the second: "no
        // layer `L2` here" was true of the surface and false of the language.
        // Every layer resolves now, so what is left to say is that this
        // particular slot has none — with what a slot holds one for, because a
        // model that reads that can decide whether to ask for a different slot.
        let none_held = slots.path(0, Kind::L2, 0).expect_err("this slot has no L2");
        assert!(none_held.contains("holds no L2"), "{none_held}");
        assert!(none_held.contains("optional"), "{none_held}");
    }

    /// **The index counts within a layer, keeping file order** — the rule
    /// `history::seed` files a snapshot under, so an address that reaches the
    /// second renderer here reaches the second renderer's versions there.
    ///
    /// The fixture interleaves the layers on purpose. Counting a file's
    /// position in the slot instead would hand back a real procedure at every
    /// address and the wrong one at most of them, which is the failure that
    /// reads as the language being confusing rather than as a resolver being
    /// wrong.
    #[test]
    fn a_node_is_indexed_within_its_own_layer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let at = dir.path();
        let head = declaring(at, "head.kir", "L1");
        let warp_a = declaring(at, "warp_a.kir", "L2");
        let sprites = declaring(at, "sprites.kir", "L4");
        let warp_b = declaring(at, "warp_b.kir", "L2");
        let strokes = declaring(at, "strokes.kir", "L4");
        let source_b = declaring(at, "source_b.kir", "L1");
        let camera = declaring(at, "camera.kir", "L3");
        let blob = declaring(at, "blob.kir", "Field");
        let slots = Slots(vec![(
            head.clone(),
            vec![
                warp_a.clone(),
                sprites.clone(),
                warp_b.clone(),
                strokes.clone(),
                source_b.clone(),
                camera.clone(),
                blob.clone(),
            ],
        )]);

        for (layer, index, expected) in [
            (Kind::L1, 0, &head),
            (Kind::L2, 0, &warp_a),
            (Kind::L4, 0, &sprites),
            (Kind::L2, 1, &warp_b),
            (Kind::L4, 1, &strokes),
            // **The head keeps L1 0**, so a second source is 1 — a chain that
            // names another geometry is another source, not a fresh count.
            (Kind::L1, 1, &source_b),
            (Kind::L3, 0, &camera),
            (Kind::Field, 0, &blob),
        ] {
            let name = layer_name(layer);
            assert_eq!(
                slots
                    .path(0, layer, index)
                    .unwrap_or_else(|e| panic!("{name}:{index}: {e}")),
                expected,
                "{name}:{index} resolved to the wrong file"
            );
        }

        let past = slots
            .path(0, Kind::L2, 2)
            .expect_err("there is no third L2");
        assert!(past.contains("0-1"), "the range is not named: {past}");
        // This slot was given one camera, so an index past it is worth saying
        // rather than folding onto the one there is — the same sentence a
        // second L2 gets, since nothing here caps a layer.
        let two_cameras = slots
            .path(0, Kind::L3, 1)
            .expect_err("this slot was given one camera");
        assert!(two_cameras.contains("one L3"), "{two_cameras}");
    }

    /// **A file that cannot be read is counted as a renderer**, which is what
    /// `history::seed` makes of one and what the compile is about to refuse it
    /// as. Guessing nothing at all would make a slot's whole chain
    /// unaddressable the moment one file in it went missing.
    #[test]
    fn an_unreadable_file_is_counted_as_a_renderer() {
        let missing = Slots(vec![(
            "nowhere/l1.kir".into(),
            vec!["nowhere/gone.kir".into()],
        )]);
        assert_eq!(
            missing.path(0, Kind::L4, 0).expect("counted as a renderer"),
            &std::path::PathBuf::from("nowhere/gone.kir")
        );
    }

    /// **A layer is parsed once, and a name this language does not have comes
    /// back with the ones it does.** A model that is told which five there are
    /// can fix its own call, which is the same reason the checker's
    /// diagnostics come back through this surface at all.
    #[test]
    fn a_layer_this_language_does_not_have_is_refused_with_the_list() {
        let refused =
            slot_layer_index(&json!({ "slot": 0, "layer": "L9" })).expect_err("there is no L9");
        assert!(refused.contains("L1, L2, L3, L4, Field"), "{refused}");

        // Case does not matter: `l1` and `field` are what a model tends to
        // type, and refusing them teaches nobody anything.
        assert_eq!(
            slot_layer_index(&json!({ "slot": 0, "layer": "l1" })).expect("l1"),
            (0, Kind::L1, 0)
        );
        assert_eq!(
            slot_layer_index(&json!({ "slot": 3, "layer": "field", "index": 0 })).expect("field"),
            (3, Kind::Field, 0)
        );
    }

    /// **A renderer is addressed by index, and an index past the stack is
    /// refused rather than folded to the first.**
    ///
    /// This surface used to hand back renderer 0 for any `L4` and say so in a
    /// comment, which was honest and useless: a model told to rewrite the
    /// streaks of a slot that draws sprites *and* streaks would have rewritten
    /// the sprites. The refusal names the range, because a model that can read
    /// the range can fix its own call — the same reason the checker's
    /// diagnostics come back through this surface rather than going to a
    /// terminal nobody is watching.
    #[test]
    fn a_renderer_is_addressed_by_index_and_a_bad_one_names_the_range() {
        let stacked = Slots(vec![(
            "a/l1.kir".into(),
            vec!["a/sprites.kir".into(), "a/strokes.kir".into()],
        )]);

        assert_eq!(
            stacked.path(0, Kind::L4, 1).expect("the second renderer"),
            &std::path::PathBuf::from("a/strokes.kir")
        );
        // Omitting it is 0, which is what every call written before stacks
        // existed means and what a slot with one renderer always means.
        assert_eq!(
            stacked.path(0, Kind::L4, 0).expect("the first renderer"),
            &std::path::PathBuf::from("a/sprites.kir")
        );

        let past = stacked
            .path(0, Kind::L4, 2)
            .expect_err("there is no third renderer");
        assert!(past.contains("0-1"), "the range is not named: {past}");

        // This slot holds one geometry, so an index on it is a mistake worth
        // saying — quietly ignoring it would let a model believe it had
        // addressed something. A slot *may* hold a second source; naming one
        // is what makes it addressable, and this fixture names none.
        let l1_indexed = stacked
            .path(0, Kind::L1, 1)
            .expect_err("this slot has one source");
        assert!(l1_indexed.contains("one L1"), "{l1_indexed}");
    }

    /// **A tool failure comes back as a result, not as a protocol error.**
    /// A model told "your call was malformed" learns nothing; one handed the
    /// checker's diagnostics can fix its own source, which is the entire loop
    /// this surface exists for.
    #[test]
    fn a_refused_write_returns_the_diagnostics_as_content() {
        let (tx, rx) = mpsc::channel();
        drop(tx);
        let mut state = state(rx);
        let request = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "name": "write_procedure",
                "arguments": { "slot": 0, "layer": "L4", "source": "not a procedure" },
            },
        });
        let reply = dispatch(&request, &mut state)
            .settled()
            .expect("a call is answered");
        let result = reply.get("result").expect("a result, not an error");
        assert_eq!(result["isError"], json!(true));
        let text = result["content"][0]["text"].as_str().expect("text");
        // The checker's words, against the source, rather than a bare failure.
        assert!(text.contains("parse"), "{text}");
        assert!(
            reply.get("error").is_none(),
            "a bad procedure must not look like a bad request"
        );
    }

    /// A notification has no `id` and is never answered — the one shape of
    /// message that must produce no reply at all.
    #[test]
    fn a_notification_is_acted_on_and_not_answered() {
        let (_tx, rx) = mpsc::channel();
        let mut state = state(rx);
        let notification = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(dispatch(&notification, &mut state).settled().is_none());
        // And one that *does* carry an id is answered, so the test above is
        // about the notification rather than about the method being unknown.
        let request = json!({ "jsonrpc": "2.0", "id": 7, "method": "ping" });
        assert!(dispatch(&request, &mut state).settled().is_some());
    }

    /// The swap history is bounded and is a report on the present.
    #[test]
    fn the_swap_history_does_not_grow_without_end() {
        let (tx, rx) = mpsc::sync_channel(RECENT * 4);
        let mut state = state(rx);
        for i in 0..(RECENT * 3) {
            tx.send(Event::Swap {
                slot: 0,
                said: format!("event {i}"),
            })
            .expect("send");
        }
        let said = swap_outcome(&mut state).expect("outcome");
        assert_eq!(state.recent.len(), RECENT);
        assert!(
            said.contains(&format!("event {}", RECENT * 3 - 1)),
            "the newest event was dropped"
        );
        assert!(!said.contains("event 0"), "the oldest event was kept");
    }

    /// **A save the render loop does not answer ends, and says something true.**
    ///
    /// Three ways a model can be left waiting, and none of them may end in a
    /// call that never returns or in a claim nobody can support. Over the
    /// channel rather than over a socket for the reason `drained_saves` is
    /// tested that way: the bound is the whole point and a render loop is not
    /// needed to see it.
    #[test]
    fn a_save_the_loop_does_not_answer_ends_and_says_something_true() {
        // Never taken. Nothing was saved and saying so is safe.
        let (kept, news) = mpsc::channel::<News>();
        let started = std::time::Instant::now();
        let said = awaited(&news, std::time::Duration::from_millis(60))
            .expect_err("a save nobody took came back as a success");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(1),
            "the wait ran past its bound"
        );
        assert!(said.contains("Nothing was saved"), "{said}");
        drop(kept);

        // Taken and named, then silence. **The id is in the answer** — that is
        // what the first message is for — and the answer claims neither success
        // nor failure.
        let (tx, news) = mpsc::channel();
        tx.send(News::Accepted(
            "slot 0: saving 2 nodes as set `keeper` in <store>".to_string(),
        ))
        .expect("accepted");
        let said = awaited(&news, std::time::Duration::from_millis(60))
            .expect_err("a save with no outcome came back as a success");
        assert!(
            said.contains("`keeper`"),
            "a timed-out save did not name the id it was accepted under: {said}"
        );
        assert!(
            said.contains("neither a success nor a failure"),
            "a timed-out save was reported as one or the other: {said}"
        );

        // The loop ended without answering: told at once rather than at the
        // deadline, which the long wait here is what proves.
        let (tx, news) = mpsc::channel();
        tx.send(News::Accepted(
            "slot 0: saving 2 nodes as set `keeper` in <store>".to_string(),
        ))
        .expect("accepted");
        drop(tx);
        let started = std::time::Instant::now();
        let said = awaited(&news, std::time::Duration::from_secs(60))
            .expect_err("a loop that ended came back as a success");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(1),
            "a client waited out the deadline on a loop that was already gone"
        );
        assert!(said.contains("`keeper`"), "{said}");
    }

    /// **The render loop's own accept reaches a client that times out**, and
    /// names the id it will find the set under.
    ///
    /// The two halves of a truthful timeout meeting for the first time:
    /// [`crate::accepted_save`] is what the loop says at the frame it takes a
    /// save, and [`awaited`] is what this server does with it. Every other test
    /// that reaches a save drives a stand-in loop which sends `accepted`
    /// *itself* — so deleting `reply.accepted` from the real loop left the whole
    /// suite green, while a client whose deadline passed was told **"Nothing was
    /// saved, and asking again is safe"** about a save that was running and
    /// would land. That is a false claim to a model about a disk, and preventing
    /// exactly it is why [`Reply`] carries two messages rather than one.
    ///
    /// The reply is deliberately still alive at the deadline: this is a slow
    /// save, not a dead loop, and the two have different answers.
    #[test]
    fn a_save_the_loop_has_taken_names_its_id_to_a_client_that_times_out() {
        let (tx, news) = mpsc::channel();
        let reply = Reply(tx);
        // One node, because the sentence counts them and a fixture that agreed
        // with a hardcoded plural would be checking the fixture.
        let sources = crate::Sources(vec![crate::SavedNode {
            layer: "L1",
            index: 0,
            hash: karakuri_store::hash::Hash::of(b"kind L1"),
            name: None,
            source: None,
            meta: None,
        }]);
        let id = crate::accepted_save(
            1,
            Some("keeper".to_string()),
            &sources,
            std::path::Path::new("/nowhere/store"),
            Some(&reply),
        );
        assert_eq!(
            id, "keeper",
            "a client's own id is what the set is filed under"
        );

        let said = awaited(&news, std::time::Duration::from_millis(60))
            .expect_err("a save with no outcome yet came back as a success");
        assert!(
            said.starts_with("slot 1: saving 1 node as set `keeper` in /nowhere/store"),
            "the loop's acceptance did not reach the client, so a timeout has no id \
             to offer: {said}"
        );
        assert!(
            said.contains("neither a success nor a failure"),
            "a save with no outcome was reported as one or the other: {said}"
        );
        assert!(
            !said.contains("Nothing was saved"),
            "a save that had been taken and is being written was reported to a model \
             as one that never happened: {said}"
        );
        drop(reply);
    }

    /// **An id from a client is one path component**, which is what a Set id is
    /// everywhere else in this program.
    #[test]
    fn a_set_id_from_a_client_is_one_path_component() {
        assert_eq!(checked_id("keeper-01"), Ok("keeper-01".to_string()));
        assert_eq!(checked_id("a_B_9"), Ok("a_B_9".to_string()));
        // What a save with no id is called, so a client can name one the same
        // way the run would have.
        let stamp = crate::history::stamped_id();
        assert_eq!(checked_id(&stamp), Ok(stamp.clone()), "{stamp}");

        for bad in [
            "../../../etc/passwd",
            "sets/../../elsewhere",
            "a/b",
            "",
            "a b",
            "night.01",
            "~/mine",
        ] {
            assert!(
                checked_id(bad).is_err(),
                "`{bad}` was accepted as the name of a file under `<store>/sets/`"
            );
        }
        assert!(checked_id(&"x".repeat(MAX_ID + 1)).is_err());

        // **The over-length refusal counts what it measures.** `str::len` is
        // bytes and the message said "characters", which agree for everything
        // that would get past the charset check and disagree for exactly the
        // caller this message exists for. Thirty-three two-byte characters is
        // sixty-six bytes, so the two readings cannot both be right here.
        let multibyte = "é".repeat(33);
        let refusal = checked_id(&multibyte).expect_err("66 bytes is past the cap");
        assert!(
            refusal.contains("is 66 bytes"),
            "an id was refused for a length its caller cannot count to: {refusal}"
        );
    }
}
