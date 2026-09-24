#![allow(unused_imports, dead_code)]

//! Wire protocol integration tests over TCP sockets.

pub use karakuri_ir::typed::Checked;
pub use karakuri_ir::Kind;
pub use karakuri_mcp::{
    serve, OperateRequest, Reporter, SaveRequest, Slots, WireRequest, LISTED, PROTOCOL,
};
pub use karakuri_operation::{Operation, SlotPolicy};
pub use karakuri_store::{Hash, Layer, Line, Record, Store};
pub use serde_json::{json, Value};
pub use std::io::{BufRead, BufReader, Write};
pub use std::sync::mpsc;

pub struct Server {
    pub port: u16,
    pub dir: tempfile::TempDir,
}

/// Helper returning the store directory within a test temporary directory.
pub fn store_root(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().join("store")
}

/// Opening state with all classes closed.
pub fn closed() -> karakuri_environment::Opening {
    karakuri_environment::Opening::closed()
}

pub fn policies() -> karakuri_environment::SlotPolicies {
    karakuri_environment::SlotPolicies::new()
}

impl Server {
    /// The library this server was started on, open from the test's side.
    pub fn store(&self) -> Store {
        Store::open(store_root(&self.dir)).expect("store")
    }
}

/// Minimal L1 test procedure.
pub const PROBE_L1: &str = r#"
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

pub const PROBE_L4: &str = r#"
proc probe_l4 {
kind  L4
blend additive

consumes position

vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.008;
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
pub const PROBE_L2: &str = r#"
proc probe_warp {
kind L2

consumes position

deform {
position = vec3(position.x, position.y * 1.5, position.z);
}
}
"#;

/// **A deformation that declares a slot nothing here can bind.**
///
/// It compiles on its own — a `uses` is a declaration, and one procedure is
/// all `compile::check` ever sees — and the Set it lands in refuses to
/// build until an `edge` says what fills `shape`. There is no tool on this
/// surface that writes one.
pub const PROBE_L2_USES: &str = r#"
proc probe_warp_uses {
kind L2

uses shape : Field

consumes position

deform {
position = position * (1.0 + shape(position) * 0.1);
}
}
"#;

pub const PROBE_L3: &str = r#"
proc probe_camera {
kind L3

camera {
eye    = vec3(0.0, 2.0, 9.0);
target = vec3(0.0, 0.0, 0.0);
}
}
"#;

pub const PROBE_FIELD: &str = r#"
proc probe_blob {
kind Field

field {
distance = sd_sphere(point, 1.0);
}
}
"#;

pub const PROBE_L1_B: &str = r#"
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

/// **A second renderer, and every number in it different.** For the test
/// that a write lands on the node its address names: two files that read
/// the same would let a write to `L4:1` land on `L4:0` and pass.
pub const PROBE_L4_B: &str = r#"
proc probe_l4_b {
kind  L4
blend additive

consumes position

vertex {
clip       = camera * vec4(position, 1.0);
point_rate = 0.02;
}

fragment {
color = vec4(0.0, 1.0, 0.0, 1.0);
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
pub const PROBE_KNOBS: &str = r#"
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
pub fn start_chain() -> Server {
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
    let reporter = serve(
        0,
        Slots::of(vec![(head, rest)]),
        store_root(&dir),
        true,
        closed(),
        policies(),
    )
    .expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);
    Server { port, dir }
}

pub fn start(watching: bool) -> Server {
    let (server, reporter) = started(watching);
    stand_in(reporter, no_loop);
    server
}

/// The same fixture with the loop's half handed back, for the tests that
/// are about what the loop says.
pub fn started(watching: bool) -> (Server, Reporter) {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1 = dir.path().join("l1.kir");
    let l4 = dir.path().join("l4.kir");
    std::fs::write(&l1, PROBE_L1).expect("l1");
    std::fs::write(&l4, PROBE_L4).expect("l4");
    // Port 0: the operating system picks, and `serve` reports what it got —
    // which is also the fix for `--mcp 0` naming a port that is not the port.
    let reporter = serve(
        0,
        Slots::of(vec![(l1, vec![l4])]),
        store_root(&dir),
        watching,
        closed(),
        policies(),
    )
    .expect("serve");
    let port = reporter.port();
    (Server { port, dir }, reporter)
}

/// Spawns a background thread acting as a mock render loop to process [`SaveRequest`]s from the server.
pub fn stand_in(reporter: Reporter, answer: impl Fn(SaveRequest) + Send + 'static) {
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
pub fn no_loop(request: SaveRequest) {
    let SaveRequest { slot, reply, .. } = request;
    reply.settled(Err(format!(
        "slot {slot}: this fixture has no render loop behind it"
    )));
}

/// One request, one reply, over TCP exactly as a client would.
pub fn post(port: u16, body: &str) -> (u16, String) {
    raw(
        port,
        &format!(
            "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    )
}

pub fn raw(port: u16, request: &str) -> (u16, String) {
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

pub fn call(port: u16, name: &str, args: Value) -> (bool, String) {
    let reply = call_raw(port, name, args);
    let result = &reply["result"];
    (
        result["isError"].as_bool().unwrap_or(true),
        result["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    )
}

pub fn call_raw(port: u16, name: &str, args: Value) -> Value {
    let (_, body) = post(
        port,
        &json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                "params":{"name":name,"arguments":args}})
        .to_string(),
    );
    serde_json::from_str(&body).expect("json")
}

// -- the listing -----------------------------------------------------

/// An artifact in the store, with or without its card — [`kept`] without
/// the Set, for the tests that write Sets of their own.
pub fn stored(server: &Server, source: &str, card: bool) -> Hash {
    let store = server.store();
    let hash = store.put_artifact(source.as_bytes()).expect("put");
    if card {
        let checked = karakuri_environment::compile::check(source).expect("the fixture compiles");
        store
            .write_meta(&hash, &karakuri_environment::meta::card(&hash, &checked))
            .expect("card");
    }
    hash
}

/// A Set file holding exactly the nodes it is given, and nothing else.
pub fn set_of(server: &Server, id: &str, nodes: &[(Layer, u32, Option<&str>, Hash)]) {
    let lines: Vec<Line> = nodes
        .iter()
        .map(|(layer, index, name, hash)| {
            Line::new(Record::Slot {
                at: karakuri_store::record::NodeAddress {
                    layer: *layer,
                    index: *index,
                },
                name: name.map(str::to_string),
                proc_hash: *hash,
            })
        })
        .collect();
    server.store().write_set(id, &lines).expect("set");
}

/// Helper setting the modified timestamp on a saved Set file for ordering tests.
pub fn written_at(server: &Server, id: &str, secs: u64) {
    let path = store_root(&server.dir)
        .join("sets")
        .join(format!("{id}.kbset"));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("open the set file");
    file.set_times(
        std::fs::FileTimes::new()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs)),
    )
    .expect("set the mtime");
}

/// Where an id appears in an answer, so a test can talk about order.
pub fn at(said: &str, id: &str) -> usize {
    said.find(&format!("`{id}`"))
        .unwrap_or_else(|| panic!("`{id}` is not in the listing at all: {said}"))
}

/// Saves an artifact into the test store with optional metadata card and Set mapping.
pub fn kept(server: &Server, id: &str, source: &str, card: bool) -> Hash {
    let store = server.store();
    let hash = store.put_artifact(source.as_bytes()).expect("put");
    if card {
        let checked = karakuri_environment::compile::check(source).expect("the fixture compiles");
        store
            .write_meta(&hash, &karakuri_environment::meta::card(&hash, &checked))
            .expect("card");
    }
    set_naming(server, id, hash);
    hash
}

/// Helper creating a minimal Set file naming a single artifact hash.
pub fn set_naming(server: &Server, id: &str, hash: Hash) {
    server
        .store()
        .write_set(
            id,
            &[
                Line::new(Record::Slot {
                    at: karakuri_store::record::NodeAddress {
                        layer: Layer::L1,
                        index: 0,
                    },
                    name: Some("shell".into()),
                    proc_hash: hash,
                }),
                Line::new(Record::Param {
                    at: Some(karakuri_store::record::NodeAddress {
                        layer: Layer::L1,
                        index: 0,
                    }),
                    key: "radius".into(),
                    value: karakuri_store::record::Value::Scalar(2.5),
                }),
            ],
        )
        .expect("set");
}

pub fn wiring_loop(
    reporter: Reporter,
) -> std::sync::Arc<std::sync::Mutex<Vec<(usize, karakuri_engine::set::Edge)>>> {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let kept = seen.clone();
    std::thread::spawn(move || loop {
        for request in reporter.saves() {
            no_loop(request);
        }
        for request in reporter.wires() {
            let WireRequest { slot, edge, reply } = request;
            let said = format!(
                "slot {slot}: `{}.{}` is bound to `{}`, and the slot is rebuilding",
                edge.node, edge.slot, edge.to
            );
            kept.lock().expect("seen").push((slot, edge));
            reply.settled(Ok(said));
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    });
    seen
}

#[allow(clippy::type_complexity)]
pub fn wired(
    watching: bool,
) -> (
    Server,
    std::sync::Arc<std::sync::Mutex<Vec<(usize, karakuri_engine::set::Edge)>>>,
) {
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
        write("blob.kir", PROBE_FIELD),
    ];
    let reporter = serve(
        0,
        Slots::of(vec![(head, rest)]),
        store_root(&dir),
        watching,
        closed(),
        policies(),
    )
    .expect("serve");
    let port = reporter.port();
    let seen = wiring_loop(reporter);
    (Server { port, dir }, seen)
}
