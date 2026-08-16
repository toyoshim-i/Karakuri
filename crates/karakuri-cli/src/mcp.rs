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
//! Only the outcome comes from the render loop, and it arrives over a channel
//! rather than a lock, so the frame path neither blocks nor waits.
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

/// The half of the server the render loop holds.
pub struct Reporter {
    sender: mpsc::SyncSender<Event>,
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

    /// The port actually bound, which is not the one asked for when that was 0.
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Where a slot's two procedures live, so a tool can name a slot rather than a
/// path.
///
/// **Paths never cross the protocol.** A client may be on another machine
/// through an `ssh -L`, where a path means nothing — and a tool that took one
/// would be inviting a model to write anywhere on the render machine's disk.
#[derive(Clone)]
pub struct Slots(pub Vec<(std::path::PathBuf, Vec<std::path::PathBuf>)>);

impl Slots {
    fn path(&self, slot: usize, layer: &str) -> Result<&std::path::PathBuf, String> {
        let pair = self
            .0
            .get(slot)
            // `len() - 1` underflowed on an empty deck and took the whole
            // surface with it: the panic unwound out of the listener thread, so
            // a process that had announced a port was silently no longer on it.
            // `serve` refuses an empty deck now; this stays correct anyway.
            .ok_or_else(|| match self.0.len() {
                0 => format!("no slot {slot}: this deck holds none"),
                n => format!("no slot {slot}: this deck holds 0-{}", n - 1),
            })?;
        match layer.to_ascii_uppercase().as_str() {
            "L1" => Ok(&pair.0),
            // **The first renderer.** A slot may draw with several now, and
            // naming one of them needs the address a param does — see
            // `docs/roadmap.md`, "How a param is addressed". Until that exists
            // this surface reaches the one it can name, and says so rather than
            // picking silently.
            "L4" => pair.1.first().ok_or_else(|| {
                format!("slot {slot} has no L4: a Set needs at least one renderer")
            }),
            other => Err(format!("no layer `{other}`: a slot holds L1 and L4")),
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

/// Serve MCP on `port`, loopback only, until the process ends.
///
/// Returns the [`Reporter`] the render loop keeps. The listener and everything
/// behind it live on threads of their own; nothing here is ever called from a
/// frame.
pub fn serve(port: u16, slots: Slots, watching: bool) -> Result<Reporter, String> {
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

    let state = std::sync::Arc::new(std::sync::Mutex::new(State {
        slots,
        watching,
        events: rx,
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
                let spawned = std::thread::Builder::new()
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
        dropped,
        port: bound.port(),
    })
}

struct State {
    slots: Slots,
    /// Whether `--watch` is on. Without it a written procedure sits on disk and
    /// changes nothing, which a model has no way to discover and every reason
    /// to be told.
    watching: bool,
    events: mpsc::Receiver<Event>,
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
fn handle(
    stream: std::net::TcpStream,
    state: &std::sync::Mutex<State>,
) -> Result<(), String> {
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
            return respond(&mut writer, 411, "text/plain", b"a POST needs a content-length");
        };
        if length > MAX_BODY {
            return respond(&mut writer, 413, "text/plain", b"that body is too large");
        }

        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;

        let reply = match serde_json::from_slice::<Value>(&body) {
            Ok(request) => {
                let mut state = state.lock().map_err(|_| "the mcp state is poisoned")?;
                dispatch(&request, &mut state)
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
fn read_capped(
    reader: &mut impl BufRead,
    into: &mut String,
) -> Result<usize, String> {
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
    writer.write_all(head.as_bytes()).map_err(|e| e.to_string())?;
    writer.write_all(body).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}

// -- the protocol ----------------------------------------------------------

/// The version of MCP this speaks.
const PROTOCOL: &str = "2024-11-05";

fn dispatch(request: &Value, state: &mut State) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    // **No `id` is a notification: never answered, and this server acts on none
    // of them.** The comment here used to say "acted on", which was false —
    // nothing below this line runs — and the test asserting `is_none()` could
    // not tell the difference. There is nothing a notification asks of this
    // server today; when there is, it goes above this line.
    let id = id?;

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
        "tools/call" => call_tool(request, state).map_err(Refused::BadParams),
        other => Err(Refused::NoMethod(format!("no method `{other}`"))),
    };

    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        // **The code says which kind of wrong.** Everything used to come back
        // as "method not found", so a client could not tell a method it had
        // invented from arguments it had got wrong.
        Err(Refused::NoMethod(m)) => error(&id, -32601, &m),
        Err(Refused::BadParams(m)) => error(&id, -32602, &m),
    })
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
    json!([
        {
            "name": "read_procedure",
            "description":
                "The source of one deck slot's procedure. `layer` is L1 (what the elements \
                 are and how they move) or L4 (how they are drawn). Read before writing: \
                 the edit is usually small, and what is already there is the best guide to \
                 the language.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": { "type": "integer", "description": "deck slot, from 0" },
                    "layer": { "type": "string", "enum": ["L1", "L4"] },
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
                    "layer": { "type": "string", "enum": ["L1", "L4"] },
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
    ])
}

fn call_tool(request: &Value, state: &mut State) -> Result<Value, String> {
    let params = request.get("params").ok_or("no params")?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("no tool name")?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let outcome = match name {
        "read_procedure" => read_procedure(&args, state),
        "write_procedure" => write_procedure(&args, state),
        "swap_outcome" => swap_outcome(state),
        other => return Err(format!("no tool `{other}`")),
    };

    // **A tool failure is a result, not a protocol error.** A model that is
    // told "the call was malformed" learns nothing; one handed the checker's
    // diagnostics can fix its own source, which is the whole loop.
    Ok(match outcome {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        Err(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": true }),
    })
}

fn slot_and_layer(args: &Value) -> Result<(usize, String), String> {
    let slot = args
        .get("slot")
        .and_then(Value::as_u64)
        .ok_or("`slot` is required and is a number")? as usize;
    let layer = args
        .get("layer")
        .and_then(Value::as_str)
        .ok_or("`layer` is required and is \"L1\" or \"L4\"")?;
    Ok((slot, layer.to_string()))
}

fn read_procedure(args: &Value, state: &State) -> Result<String, String> {
    let (slot, layer) = slot_and_layer(args)?;
    let path = state.slots.path(slot, &layer)?;
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_procedure(args: &Value, state: &State) -> Result<String, String> {
    let (slot, layer) = slot_and_layer(args)?;
    let source = args
        .get("source")
        .and_then(Value::as_str)
        .ok_or("`source` is required")?;
    let path = state.slots.path(slot, &layer)?.clone();

    // **Checked before it is written, and the diagnostics are handed back.**
    // Writing first and letting the watcher report would put the compiler's
    // answer on a terminal the model cannot see.
    let checked = crate::compile::check(source)?;
    let expected = match layer.to_ascii_uppercase().as_str() {
        "L1" => karakuri_ir::Kind::L1,
        _ => karakuri_ir::Kind::L4,
    };
    if checked.kind != expected {
        return Err(format!(
            "this is a {:?} procedure and slot {slot}'s {layer} is a {expected:?} — \
             the two layers are not interchangeable",
            checked.kind
        ));
    }

    // **What else this write reaches.** `watch.rs` documents two slots sharing a
    // pair as supported, and the manual's own example gives one `soft_points.kir`
    // to three slots — so naming one slot was reporting a third of what
    // happened. Anything skipped or widened is said with a count.
    let also: Vec<String> = (0..state.slots.0.len())
        .filter(|other| *other != slot)
        .filter(|other| {
            ["L1", "L4"]
                .iter()
                .any(|l| state.slots.path(*other, l).is_ok_and(|p| *p == path))
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
    Ok(if state.watching {
        format!(
            "compiled and written to slot {slot} {layer}.{shared} It is being built on a \
             worker thread and will swap in at a frame boundary; call `swap_outcome` to \
             find out whether it landed or was rolled back for cost.\n\n\
             **This replaced the file on disk and there is no backup.** If you did not read \
             it first, the previous version is gone."
        )
    } else {
        format!(
            "compiled and written to slot {slot} {layer}.{shared} **This run was started \
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
        format!("\n\n({dropped} earlier report{} were dropped for want of room — this is \
                 not the whole history)", if dropped == 1 { "" } else { "s" })
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
    out.push_str("\n# Topologies\n\nDeclared by an L1's `topology`. What a *renderer* draws \
                  is not declared: an L4 draws segments when its `vertex` block assigns \
                  `clip_b` and sprites when it does not.\n\n");
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

    out.push_str("\n# Stage outputs\n\nAssigned like attributes; reading one is an error. \
                  `clip` and `point_size` are required in a `vertex` block, and `color` in a \
                  `fragment` block, on every path through it — but **a `vertex` block is \
                  itself optional**, which is how an L4 says it draws the whole frame. \
                  `clip_b` is the one optional output, and assigning it on only some paths \
                  is rejected.\n\n");
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

    fn start(watching: bool) -> Server {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        let l4 = dir.path().join("l4.kir");
        std::fs::write(&l1, PROBE_L1).expect("l1");
        std::fs::write(&l4, PROBE_L4).expect("l4");
        // Port 0: the operating system picks, and `serve` reports what it got —
        // which is also the fix for `--mcp 0` naming a port that is not the port.
        let reporter = serve(0, Slots(vec![(l1, vec![l4])]), watching).expect("serve");
        let port = reporter.port();
        // Held for the life of the test: dropping it closes the channel.
        std::mem::forget(reporter);
        Server { port, dir }
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
        reader.read_line(&mut status_line).expect("status");
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
            result["content"][0]["text"].as_str().unwrap_or("").to_string(),
        )
    }

    /// The loop the whole surface exists for: read, write, and the file changes.
    #[test]
    fn a_procedure_can_be_read_and_rewritten_over_the_wire() {
        let server = start(true);
        let (failed, source) = call(server.port, "read_procedure", json!({"slot":0,"layer":"L4"}));
        assert!(!failed, "{source}");
        assert!(source.contains("proc probe_l4"), "{}", &source[..80.min(source.len())]);

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
        let (_, l1) = call(server.port, "read_procedure", json!({"slot":0,"layer":"L1"}));
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
        let (status, _) = post(server.port, &json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string());
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
        let (status, _) = post(server.port, &json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string());
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
        assert!(!body.contains("\"id\":99"), "the smuggled request ran: {body}");
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
            &format!("POST / HTTP/1.1\r\nCONTENT-LENGTH: {}\r\n\r\n{body}", body.len()),
        );
        assert_eq!(status, 200, "an uppercase header name was not understood");
        assert!(reply.contains("result"), "{reply}");

        let (status, said) = raw(server.port, "POST / HTTP/1.1\r\nContent-Length: 12x\r\n\r\n");
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
        let (status, _) = post(server.port, &json!({"jsonrpc":"2.0","id":1,"method":"ping"}).to_string());
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
        let reporter = serve(0, shared, true).expect("serve");
        let port = reporter.port();
        std::mem::forget(reporter);

        let (_, source) = call(port, "read_procedure", json!({"slot":0,"layer":"L4"}));
        let (failed, said) = call(
            port,
            "write_procedure",
            json!({"slot":0,"layer":"L4","source":source}),
        );
        assert!(!failed, "{said}");
        assert!(said.contains("also slot 1"), "the other slot was not named: {said}");
        assert!(said.contains("no backup"), "the destruction was not named: {said}");
    }

    /// The tools and resources a client is offered are the ones that answer.
    #[test]
    fn everything_advertised_can_be_called() {
        let server = start(true);
        let (_, listed) = post(server.port, &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string());
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
            assert!(!said.is_empty(), "`{name}` is advertised and answers nothing");
        }

        let (_, listed) =
            post(server.port, &json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}).to_string());
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
            let text = reply["result"]["contents"][0]["text"].as_str().unwrap_or("");
            assert!(text.len() > 100, "`{uri}` is advertised and reads as nothing");
        }
    }

    /// `initialize` answers with what a client needs to proceed.
    #[test]
    fn initialize_answers_with_a_protocol_version_and_capabilities() {
        let server = start(true);
        let (status, body) =
            post(server.port, &json!({"jsonrpc":"2.0","id":1,"method":"initialize"}).to_string());
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
        let (_, source) = call(server.port, "read_procedure", json!({"slot":0,"layer":"L4"}));
        let (failed, said) = call(
            server.port,
            "write_procedure",
            json!({"slot":0,"layer":"L4","source":source}),
        );
        assert!(!failed, "{said}");
        assert!(said.contains("--watch"), "{said}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slots() -> Slots {
        Slots(vec![
            ("a/l1.kir".into(), vec!["a/l4.kir".into()]),
            ("b/l1.kir".into(), vec!["b/l4.kir".into()]),
        ])
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
            let name = line.trim_start_matches("| `").split('`').next().expect("a name");
            assert!(
                karakuri_ir::Output::from_name(name).is_some(),
                "the vocabulary lists an output `{name}` the checker does not know"
            );
        }
    }

    /// A slot is a number and a layer is one of two names. **No path crosses
    /// the protocol**: a client may be on another machine through an `ssh -L`,
    /// where a path means nothing — and a tool that took one would invite a
    /// model to write anywhere on the render machine's disk.
    #[test]
    fn a_slot_and_a_layer_resolve_and_anything_else_is_refused() {
        let slots = slots();
        assert_eq!(
            slots.path(1, "L4").expect("slot 1 L4"),
            &std::path::PathBuf::from("b/l4.kir")
        );
        assert_eq!(
            slots.path(0, "l1").expect("case does not matter"),
            &std::path::PathBuf::from("a/l1.kir")
        );

        let past_the_end = slots.path(2, "L1").expect_err("slot 2 does not exist");
        assert!(past_the_end.contains("0-1"), "{past_the_end}");

        let no_such_layer = slots.path(0, "L2").expect_err("there is no L2 here");
        assert!(no_such_layer.contains("L1 and L4"), "{no_such_layer}");
    }

    /// **A tool failure comes back as a result, not as a protocol error.**
    /// A model told "your call was malformed" learns nothing; one handed the
    /// checker's diagnostics can fix its own source, which is the entire loop
    /// this surface exists for.
    #[test]
    fn a_refused_write_returns_the_diagnostics_as_content() {
        let (tx, rx) = mpsc::channel();
        drop(tx);
        let mut state = State {
            slots: slots(),
            watching: true,
            events: rx,
            recent: Vec::new(),
            dropped: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        };
        let request = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {
                "name": "write_procedure",
                "arguments": { "slot": 0, "layer": "L4", "source": "not a procedure" },
            },
        });
        let reply = dispatch(&request, &mut state).expect("a call is answered");
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
        let mut state = State {
            slots: slots(),
            watching: true,
            events: rx,
            recent: Vec::new(),
            dropped: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        };
        let notification = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(dispatch(&notification, &mut state).is_none());
        // And one that *does* carry an id is answered, so the test above is
        // about the notification rather than about the method being unknown.
        let request = json!({ "jsonrpc": "2.0", "id": 7, "method": "ping" });
        assert!(dispatch(&request, &mut state).is_some());
    }

    /// The swap history is bounded and is a report on the present.
    #[test]
    fn the_swap_history_does_not_grow_without_end() {
        let (tx, rx) = mpsc::sync_channel(RECENT * 4);
        let mut state = State {
            slots: slots(),
            watching: true,
            events: rx,
            recent: Vec::new(),
            dropped: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        };
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
}
