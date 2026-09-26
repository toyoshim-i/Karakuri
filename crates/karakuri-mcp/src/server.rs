use std::io::Read;
use std::sync::mpsc;

use serde_json::{json, Value};

use crate::*;

/// Serves MCP on `port` over loopback until the process ends.
///
/// Returns the [`Reporter`] handle for the render loop to send notifications and events.
/// The server runs on dedicated threads decoupled from frame rendering.
pub fn serve(
    port: u16,
    slots: Slots,
    store: std::path::PathBuf,
    watching: bool,
    opening: karakuri_environment::Opening,
    slot_policies: karakuri_environment::SlotPolicies,
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
        slot_policies,
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
                // Spawns a dedicated worker thread per incoming TCP connection.
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

/// Handles an incoming TCP connection, processing HTTP POST JSON-RPC requests.
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
        // Handles routing before verb checking to respond 404 to unserved paths like oauth discovery.
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
            // Header names are matched case-insensitively per HTTP specification.
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

        // Validate origin: reject non-local Origin headers to prevent cross-origin POSTs.
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

        // Returns 404 for unknown endpoints to clarify lack of authorization services.
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
            // Responds 405 Method Not Allowed with JSON body on non-POST requests.
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
                // Releases state lock prior to waiting on asynchronous loop replies.
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

/// A JSON-RPC reply or pending asynchronous operation to be awaited with [`State`] unlocked.
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
    /// Asynchronous pending operations awaiting engine application or save completion.
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
            // Append note only on success to keep error responses focused on the refusal reason.
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
    // Notifications (requests without an `id`) are discarded without a response.
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
            // Return pending work to release lock before awaiting completion.
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
        // Map error variants to distinct JSON-RPC error codes.
        Err(Refused::NoMethod(m)) => error(&id, -32601, &m),
        Err(Refused::BadParams(m)) => error(&id, -32602, &m),
    }))
}
