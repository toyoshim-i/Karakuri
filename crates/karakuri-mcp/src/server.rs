use std::io::Read;
use std::sync::mpsc;

use serde_json::{json, Value};

use crate::*;

/// Serve MCP on `port`, loopback only, until the process ends.
///
/// Returns the [`Reporter`] the render loop keeps. The listener and everything
/// behind it live on threads of their own; nothing here is ever called from a
/// frame.
///
/// `store` is a root and not an open [`Store`], which is the same decision
/// [`Slots`] makes about a path and for a milder version of the same reason.
/// Opening here would fail a whole run for a library nothing has asked for yet,
/// and would hold one answer to "where is the store" against a directory the
/// operator is free to move; opening per call is four `create_dir_all`s off a
/// frame path, on a surface where the expensive thing is already a compile.
///
/// `slots` is a live handle and is *shared* rather than moved, exactly as
/// `opening` beside it is: the host goes on writing it every time it re-points
/// a slot, and this server resolves through it on every call. A run that hands
/// this a layout taken at launch and then loads a Set onto a deck answers a
/// model about the material it stopped running — see [`Slots`].
pub fn serve(
    port: u16,
    slots: Slots,
    store: std::path::PathBuf,
    watching: bool,
    opening: karakuri_environment::Opening,
) -> Result<Reporter, String> {
    if slots.count() == 0 {
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
    // The edges, on a channel of their own — see [`Reporter::wires`].
    let (wiring, wires) = mpsc::sync_channel(ASKED);
    // And the operations, on a third — see [`Reporter::operations`].
    let (operating, operations) = mpsc::sync_channel(ASKED);

    let state = std::sync::Arc::new(std::sync::Mutex::new(State {
        slots,
        store,
        watching,
        opening,
        events: rx,
        asked,
        wiring,
        operating,
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
        wires,
        operations,
        dropped,
        port: bound.port(),
    })
}

// -- the transport ---------------------------------------------------------

/// One connection: read requests, answer them, keep it open.
///
/// Hand-rolled, and what that costs is worth stating rather than assuming. The
/// first version argued it was fine because this is "loopback, from one
/// client". Loopback is not a boundary — any process on the machine reaches it,
/// and so does a `fetch()` from any web page the operator happens to have open,
/// because a POST with a plain content type needs no preflight. "One client"
/// was an assumption about the *good* client and nothing enforced it. What
/// enforces anything now: an `Origin` check, a body cap before any allocation,
/// a read timeout, and a thread per connection.
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

// -- the protocol ----------------------------------------------------------

/// A reply, or the one step of a reply that must happen with [`State`]
/// unlocked.
///
/// This type is the whole of the concurrency design, so it is worth stating
/// plainly what it buys. `handle` locks the state around [`dispatch`], and
/// there is a thread per connection: anything waited for under that lock is
/// waited for by every other client too. Five of the six tools are a file read
/// or a file write and finish under it — `list_sets` is the widest of them, a
/// directory read plus a set file each and a card for each node those files
/// left unnamed, which is a bounded count of reads off the store rather than a
/// wait on anybody else's thread, and is why it is capped and why it compiles
/// nothing; `read_set` is the same shape over one set. `save_set` waits for a
/// render loop and then for a disk, which is unbounded in the only sense that
/// matters — it depends on somebody else's frame rate.
///
/// So the send happens under the lock, where the channel is, and the *wait*
/// comes back out here. Returning a value that still has work in it is the
/// smallest thing that makes the boundary visible: a comment saying "do not
/// wait here" would be a comment.
///
/// `wire_input` is the second thing that waits, and it waits for a frame rather
/// than for a disk — which is shorter and is still somebody else's thread, so
/// it belongs out here for exactly the same reason.
pub(crate) enum Pending {
    /// Nothing left to do. `None` is a notification, which is answered with no body
    /// at all.
    Done(Option<Value>),
    /// A save the render loop has been asked for, and the JSON-RPC id it is
    /// answered under.
    Saving {
        id: Value,
        news: mpsc::Receiver<News>,
    },
    /// An edge the render loop has been asked to write.
    ///
    /// A variant of its own rather than a second `Saving`, because the two wait
    /// different lengths for differently shaped news — see [`WIRE_REPLY`] against
    /// [`SAVE_REPLY`], and [`applied`] against [`awaited`]. An operation the render
    /// loop has been asked to perform.
    ///
    /// [`Pending::Wiring`]'s shape with no note, and it waits with [`applied`] for
    /// that variant's reason: the loop performs it at the frame it takes it and
    /// answers there, and everything slow that an operation starts — a rebuild, a
    /// transition, a save — happens after the answer and is reported where it
    /// lands.
    Operating {
        id: Value,
        news: mpsc::Receiver<News>,
    },
    Wiring {
        id: Value,
        news: mpsc::Receiver<News>,
        /// What this server knows about the run that the loop's own sentence will not
        /// say — today, that a run without `--watch` has no watcher to rebuild the
        /// slot. Built under the lock, where [`State`] is; appended to an answer the
        /// loop wrote, because it is a fact about the run rather than about the edge.
        note: String,
    },
}

impl Pending {
    /// The reply, waiting for the render loop if that is what is left.
    ///
    /// Called with the state unlocked, which is the entire reason this type exists
    /// — see above.
    pub(crate) fn settled(self) -> Option<Value> {
        match self {
            Pending::Done(reply) => reply,
            Pending::Saving { id, news } => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": tool_result(awaited(&news, SAVE_REPLY)),
            })),
            Pending::Operating { id, news } => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": tool_result(applied(&news, OPERATE_REPLY)),
            })),
            // **The note is appended to what the loop said and only where the
            // loop said it worked.** A refusal is the loop's whole sentence;
            // adding "and by the way this run does not rebuild" to it would put
            // two answers in front of a model that has one mistake to fix.
            Pending::Wiring { id, news, note } => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": tool_result(applied(&news, WIRE_REPLY).map(|said| {
                    if note.is_empty() {
                        said
                    } else {
                        format!("{said}\n\n{note}")
                    }
                })),
            })),
        }
    }
}

pub(crate) fn dispatch(request: &Value, state: &mut State) -> Pending {
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
            Ok(Called::Wiring { news, note }) => return Pending::Wiring { id, news, note },
            Ok(Called::Operating(news)) => return Pending::Operating { id, news },
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
