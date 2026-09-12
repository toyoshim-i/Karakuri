//! **Over a socket, because everything else here passed with the server
//! deleted.** A review mutated this module twelve ways — `handle` returning
//! immediately, `serve` never binding, `respond` writing nothing,
//! `read_procedure` returning a constant, `tools()` returning `[]` — and the
//! suite was green for all twelve. A socket server whose tests never open a
//! socket is not tested.

use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;
use karakuri_mcp::{
    serve, OperateRequest, Reporter, SaveRequest, Slots, WireRequest, LISTED, PROTOCOL,
};
use karakuri_operation::Operation;
use karakuri_store::{Hash, Layer, Line, Record, Store};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc;

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

/// **What a run starts with: all four classes closed.**
///
/// Every fixture in this module serves under it, which is what makes
/// `the_seven_tools_still_work_with_every_class_closed` a property of the
/// whole file rather than of one test: if the gate had caught any of the
/// seven, this module would be red from end to end.
fn closed() -> karakuri_environment::Opening {
    karakuri_environment::Opening::closed()
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
const PROBE_L2: &str = r#"
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
const PROBE_L2_USES: &str = r#"
proc probe_warp_uses {
kind L2

uses shape : Field

consumes position

deform {
position = position * (1.0 + shape(position) * 0.1);
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

/// **A second renderer, and every number in it different.** For the test
/// that a write lands on the node its address names: two files that read
/// the same would let a write to `L4:1` land on `L4:0` and pass.
const PROBE_L4_B: &str = r#"
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
    let reporter = serve(
        0,
        Slots::of(vec![(head, rest)]),
        store_root(&dir),
        true,
        closed(),
    )
    .expect("serve");
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
    let reporter = serve(
        0,
        Slots::of(vec![(l1, vec![l4])]),
        store_root(&dir),
        watching,
        closed(),
    )
    .expect("serve");
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

/// **A re-point moves what the server resolves, on the very next call.**
///
/// This is the whole of why [`Slots`] is a handle. The server was handed
/// the launch working copies and kept them for the run, so after the panel
/// loaded a Set onto a deck — which writes new scratch files and points
/// that slot's watcher at them — every address this surface resolved was
/// the layout the deck had stopped running. A `read_procedure` handed back
/// the material the operator had just replaced and a `write_procedure`
/// wrote a file no watcher was polling, **both of them answering
/// successfully**: the wrong answer arrives as a sentence saying it worked
/// (`docs/principles/0094-…`).
///
/// So the assertion is over the wire, on both tools, before and after one
/// re-point — and the write is read back off the **new** file, because a
/// write that went to the old one would still have said *compiled and
/// written*.
///
/// Watched to fail against the launch copy: a `Slots` that answers out of
/// what it was constructed with reads back `probe_l4` after the re-point
/// and leaves `after.kir` untouched on disk.
#[test]
fn a_re_point_moves_what_an_address_resolves_to() {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let launch_l1 = write("l1.kir", PROBE_L1);
    let launch_l4 = write("l4.kir", PROBE_L4);
    let slots = Slots::of(vec![(launch_l1, vec![launch_l4])]);
    let reporter = serve(0, slots.clone(), store_root(&dir), true, closed()).expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    let (failed, source) = call(port, "read_procedure", json!({"slot":0,"layer":"L4"}));
    assert!(!failed, "{source}");
    assert!(
        source.contains("proc probe_l4"),
        "the launch layout is not what the server started on"
    );

    // Where a load leaves a slot: new files, and one aim naming them. The
    // handle is written from the aim rather than from these paths, because
    // that walk is `Slots::re_point`'s and there is one of it.
    let after_l1 = write("after_l1.kir", PROBE_L1_B);
    let after_l4 = write("after.kir", PROBE_L4_B);
    slots.re_point(
        0,
        &karakuri_environment::watch::Aim {
            head: karakuri_environment::compile::Named::bare(&after_l1),
            rest: vec![karakuri_environment::compile::Named::bare(&after_l4)],
            layering: karakuri_engine::set::Layering::Overdraw,
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
            set: Some("night02".to_owned()),
        },
    );

    let (failed, source) = call(port, "read_procedure", json!({"slot":0,"layer":"L4"}));
    assert!(!failed, "{source}");
    assert!(
        source.contains("proc probe_l4_b"),
        "the read resolved against the layout the deck stopped running, which said: {}",
        source.lines().find(|l| l.starts_with("proc")).unwrap_or("")
    );

    let edited = format!("// Edited after the load.\n{source}");
    let (failed, said) = call(
        port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","source":edited}),
    );
    assert!(!failed, "{said}");
    assert!(
        std::fs::read_to_string(dir.path().join("after.kir"))
            .expect("read back")
            .contains("Edited after the load."),
        "the write said it landed and went to the file the slot no longer runs"
    );
}

/// **A node the slot stopped holding is refused naming what it holds
/// now.**
///
/// The other half of the same defect, and the one that is refused rather
/// than answered — which makes it the *milder* half and still a wrong
/// sentence: a model told `slot 0 holds 2 L4 nodes, so index is 0-1` after
/// a load that left the slot one renderer will keep addressing a node that
/// is not there. The refusal is derived from the current nodes because
/// [`Slots::path`] walks them on every call
/// (`docs/principles/0083-…`).
///
/// Watched to fail against the launch copy: `L4:1` resolves and the write
/// lands on a file the deck is not running.
#[test]
fn a_node_a_load_took_away_is_refused_naming_what_the_slot_holds_now() {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let slots = Slots::of(vec![(
        write("l1.kir", PROBE_L1),
        vec![
            write("sprites.kir", PROBE_L4),
            write("strokes.kir", PROBE_L4_B),
        ],
    )]);
    // Two renderers at launch, so `L4:1` is a real address before the load
    // and the assertion below is a change rather than a constant.
    assert_eq!(
        slots.file(0, "L4", 1).expect("two renderers at launch"),
        dir.path().join("strokes.kir")
    );

    slots.re_point(
        0,
        &karakuri_environment::watch::Aim {
            head: karakuri_environment::compile::Named::bare(dir.path().join("l1.kir")),
            rest: vec![karakuri_environment::compile::Named::bare(
                dir.path().join("sprites.kir"),
            )],
            layering: karakuri_engine::set::Layering::Overdraw,
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
            set: Some("night02".to_owned()),
        },
    );

    let refused = slots
        .file(0, "L4", 1)
        .expect_err("the loaded Set holds one renderer");
    assert_eq!(refused, "slot 0 holds one L4 and `index` is 1");
    let refused = slots
        .file(0, "L2", 0)
        .expect_err("the loaded Set holds no deformation");
    assert!(
        refused.starts_with("slot 0 holds no L2:"),
        "the refusal is not about what the slot holds now: {refused}"
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
    // **Five, where `Kind` is six** — see `LAYERS`. `kind L5` compiles and
    // lowers and is not a layer a *slot* holds, so this list is the one a
    // client can address rather than the one the language has.
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
    let shared = Slots::of(vec![(l1.clone(), vec![l4.clone()]), (l1, vec![l4])]);
    let reporter = serve(0, shared, store_root(&dir), true, closed()).expect("serve");
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
    let shared = Slots::of(vec![
        (l1.clone(), vec![warp.clone(), l4.clone()]),
        (l1, vec![warp, l4]),
    ]);
    let reporter = serve(0, shared, store_root(&dir), true, closed()).expect("serve");
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
    let reporter = serve(
        0,
        Slots::of(vec![pair.clone(), pair]),
        store_root(&dir),
        true,
        closed(),
    )
    .expect("serve");
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
        reply.settled(Err(karakuri_environment::nothing_to_save(
            slot,
            Some("night01"),
            true,
        )));
    });
    let (failed, said) = call(server.port, "save_set", json!({"slot":0}));
    assert!(failed, "a refusal came back as a success: {said}");
    assert_eq!(
        said,
        karakuri_environment::nothing_to_save(0, Some("night01"), true),
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
        karakuri_environment::no_such_slot(7, 1),
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
    assert_eq!(said, karakuri_environment::no_such_slot(7, 1));
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
    let waiting = std::thread::spawn(move || call(port, "save_set", json!({"slot":0,"id":"slow"})));
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

// -- the listing -----------------------------------------------------

/// An artifact in the store, with or without its card — [`kept`] without
/// the Set, for the tests that write Sets of their own.
fn stored(server: &Server, source: &str, card: bool) -> Hash {
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
fn set_of(server: &Server, id: &str, nodes: &[(Layer, u32, Option<&str>, Hash)]) {
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

/// **Say when a Set was written**, so a test of the order does not depend on
/// how fast a machine writes two files.
///
/// A Set file carries no time — that is what `StoreError::TickInSet` exists
/// to enforce — so the mtime is the only record there is of when one was
/// saved, and setting it is how a fixture states the fact the listing sorts
/// on. Two Sets given the *same* second is the case worth building on
/// purpose: it is what a coarse filesystem clock produces, and it is the
/// case the tie-break exists for.
fn written_at(server: &Server, id: &str, secs: u64) {
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
fn at(said: &str, id: &str) -> usize {
    said.find(&format!("`{id}`"))
        .unwrap_or_else(|| panic!("`{id}` is not in the listing at all: {said}"))
}

/// **The tool is offered, and the library comes back most recent first.**
///
/// The two halves are one test for the reason the `read_set` pair are: a
/// tool a client is never told about and a tool that answers nothing are
/// both invisible, and this is the pass that says a model can find it and
/// use it in one go.
///
/// **`beta` and `gamma` are written in the same second on purpose.** The
/// store's own order is by id and is total; this surface sorts by recency,
/// and a sort on a coarse clock's seconds has ties — so the tie-break by id
/// is the whole reason two calls on an unchanged store say the same thing.
/// Without it these two would come back in whatever order `read_dir` felt
/// like, which is not an order at all.
#[test]
fn the_listing_is_offered_and_comes_back_most_recent_first() {
    let server = start(true);
    let hash = stored(&server, PROBE_L1, true);
    for id in ["alpha", "beta", "gamma"] {
        set_of(&server, id, &[(Layer::L1, 0, Some("shell"), hash)]);
    }
    written_at(&server, "alpha", 1_000);
    written_at(&server, "beta", 2_000);
    written_at(&server, "gamma", 2_000);

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
        names.iter().any(|name| name == "list_sets"),
        "a client is never told the tool exists: {names:?}"
    );

    let (failed, said) = call(server.port, "list_sets", json!({}));
    assert!(!failed, "{said}");
    assert!(
        at(&said, "beta") < at(&said, "alpha") && at(&said, "gamma") < at(&said, "alpha"),
        "the oldest set is not last: what did I just save is the question this is \
         mostly asked, and the answer is the wrong way round: {said}"
    );
    assert!(
        at(&said, "beta") < at(&said, "gamma"),
        "two sets written in one second came back in an order their ids do not \
         decide, so two calls on an unchanged store can disagree: {said}"
    );
}

/// **A walk answers one Set's versions, and a row filed under another Set
/// or under none is never in it.**
///
/// The narrowing is this surface's — `history::list` hands over one ordered
/// listing with the whole address on every row — so the filter is written
/// here and has to be checked here. **The `None` row is the half that
/// matters**: a version written while a slot was running material nobody
/// had saved is a version of *nothing*, and a filter that let it through
/// would be inventing a history for whichever Set was asked about
/// (ADR-0276, ADR-0308).
///
/// **Watched to fail** against three defects: a narrowing on
/// `version.set.is_none() || version.set.as_deref() == Some(id)`, which is
/// the wildcard reading and puts `bend` in the answer; a walk that returned
/// the whole listing, which puts `drift_shell` in it; and a `set` argument
/// read as optional, which answers a call that named no Set with somebody
/// else's edits.
#[test]
fn a_walk_answers_one_sets_versions_and_never_a_row_filed_under_another() {
    let server = start(true);
    let root = store_root(&server.dir);
    let mut snaps = karakuri_environment::history::Snapshots::new(&root);
    for (slot, layer, set, name, source) in [
        (0usize, "L4", Some("night01"), "beat_strokes", &b"one"[..]),
        (0, "L4", Some("night01"), "beat_strokes", &b"two"[..]),
        (1, "L1", Some("day02"), "drift_shell", &b"three"[..]),
        // **Filed under no Set**, which is a run playing the pair it was
        // launched with — the row no id matches.
        (2, "L2", None, "bend", &b"four"[..]),
    ] {
        snaps
            .record(slot, layer, 0, set, name, source)
            .expect("a snapshot is written");
    }

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
        names.iter().any(|name| name == "walk_history"),
        "a client is never told the tool exists: {names:?}"
    );

    let (failed, said) = call(server.port, "walk_history", json!({"set":"night01"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("beat_strokes"),
        "the Set's own versions are not in its walk: {said}"
    );
    assert!(
        !said.contains("drift_shell"),
        "another Set's version is in this one's history: {said}"
    );
    assert!(
        !said.contains("bend"),
        "a version filed under no Set was folded into a Set's history, which is the \
         wildcard reading ADR-0276 refuses: {said}"
    );
    // **A row is the name a landing names it back by**, which is the name
    // less the `@<set>` every row of one walk shares.
    assert!(
        !said.contains("@night01") && !said.contains(".kir"),
        "a row is not the name `Revision::Picked` takes: {said}"
    );

    let (failed, said) = call(server.port, "walk_history", json!({"set":"day02"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("drift_shell") && !said.contains("beat_strokes"),
        "the other Set's walk is not its own: {said}"
    );

    // **A Set with no versions is an answer and not a failure**, and it is
    // a different answer from a store with no history at all.
    let (failed, said) = call(server.port, "walk_history", json!({"set":"nothing_here"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("no version of set `nothing_here`"),
        "an empty walk does not say it is empty: {said}"
    );

    // **And a call that names no Set is refused rather than answered with
    // whatever the store holds.**
    let (failed, said) = call(server.port, "walk_history", json!({}));
    assert!(failed, "a walk with no Set was answered: {said}");
    assert!(said.contains("`set` is required"), "{said}");
}

/// **Both filters, apart and together.**
///
/// `holds` is the "which of these use `drift_shell`" question and `layer` is
/// the "which of these deform something" one, and the pair is the reason
/// each is a filter rather than something a reader does by eye over twenty
/// lines. Case is folded because a model that read a name in one answer and
/// typed it back with a capital is asking the same question.
///
/// **Together they are asked of the set and not of one node.** `holds` and
/// `layer` matching the same node would answer a question nobody has — the
/// useful one is *which of the sets built on this also deform something*,
/// and there the deformation is a different node with a different name.
#[test]
fn the_filters_narrow_the_listing_and_can_be_combined() {
    let server = start(true);
    let l1 = stored(&server, PROBE_L1, true);
    let l2 = stored(&server, PROBE_L2, true);
    // **Capitals in the fixture's own name and not only in the query.**
    // Folding one side and not the other passes any fixture where the
    // stored name is already lowercase, which is most of them — so the name
    // the set carries is spelled the way an operator types a name and the
    // query is spelled the way a model shouts one.
    set_of(&server, "plain", &[(Layer::L1, 0, Some("Drift_Shell"), l1)]);
    set_of(
        &server,
        "warped",
        &[
            (Layer::L1, 0, Some("Drift_Shell"), l1),
            (Layer::L2, 0, Some("bend"), l2),
        ],
    );
    set_of(
        &server,
        "other",
        &[
            (Layer::L1, 0, Some("lattice"), l1),
            (Layer::L2, 0, Some("bend"), l2),
        ],
    );

    // Case folded, and the name typed back the way a model would shout it.
    let (failed, said) = call(server.port, "list_sets", json!({"holds":"DRIFT_shell"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("`plain`") && said.contains("`warped`") && !said.contains("`other`"),
        "`holds` did not select on what the nodes are called: {said}"
    );

    let (failed, said) = call(server.port, "list_sets", json!({"layer":"L2"}));
    assert!(!failed, "{said}");
    assert!(
        said.contains("`warped`") && said.contains("`other`") && !said.contains("`plain`"),
        "`layer` did not select on the layers a set holds: {said}"
    );

    let (failed, said) = call(
        server.port,
        "list_sets",
        json!({"holds":"drift","layer":"L2"}),
    );
    assert!(!failed, "{said}");
    assert!(
        said.contains("`warped`") && !said.contains("`plain`") && !said.contains("`other`"),
        "both filters given did not mean both must hold: {said}"
    );

    // A layer nothing spells is refused with the list, as every other tool
    // refuses one — not answered as though it had matched nothing.
    let (failed, said) = call(server.port, "list_sets", json!({"layer":"L9"}));
    assert!(
        failed,
        "a layer this language does not have was accepted: {said}"
    );
    assert!(
        said.contains("Field"),
        "the refusal does not say what the layers are: {said}"
    );
}

/// **A capped listing says what it dropped.**
///
/// A library is not bounded by anything — a run that presses `k` between
/// takes keeps one a minute — so an answer that rendered whatever it found
/// would eventually be an answer nobody can read. The cap is not the
/// interesting half: a model told "here are your sets" over twenty of
/// twenty-five will tell its user they have twenty and then act on a
/// library it has not seen. So the count that matched, the count shown and
/// the fact that the filters narrow it are all in the text.
#[test]
fn a_capped_listing_can_never_be_read_as_the_whole_library() {
    let server = start(true);
    let hash = stored(&server, PROBE_L1, true);
    let total = LISTED + 5;
    for n in 0..total {
        let id = format!("set{n:02}");
        set_of(&server, &id, &[(Layer::L1, 0, Some("shell"), hash)]);
        written_at(&server, &id, 1_000 + n as u64);
    }

    let (failed, said) = call(server.port, "list_sets", json!({}));
    assert!(!failed, "{said}");
    let listed = (0..total)
        .filter(|n| said.contains(&format!("`set{n:02}`")))
        .count();
    assert_eq!(
        listed, LISTED,
        "the cap did not hold: {listed} of {total} sets were rendered\n{said}"
    );
    for expected in [
        &format!("{total} sets"),
        &format!("the {LISTED} most recently written"),
        "5 more matched and are not listed",
        "Narrow it with `holds`",
    ] {
        assert!(
            said.contains(expected),
            "a truncated listing does not say `{expected}`, so it reads as the whole \
             library: {said}"
        );
    }
    // The most recent survive the cap, because the newest is what the
    // question was about.
    assert!(
        said.contains(&format!("`set{:02}`", total - 1))
            && !said.contains(&format!("`set{:02}`", 0)),
        "the cap kept the wrong end of the library: {said}"
    );
}

/// **An empty store and a filter that matches nothing are both answers, and
/// they are different answers.**
///
/// Neither is an error: a store nobody has saved into is what every store
/// starts as, and a filter that selects none of twenty sets is the filter
/// doing its job. They read differently because they send a reader to
/// different places — one to `save_set`, the other to a different filter —
/// and being told "nothing matches" by an empty library is being told to go
/// looking for material that was never there.
#[test]
fn an_empty_store_and_a_filter_that_matches_nothing_read_differently() {
    let server = start(true);
    let (failed, said) = call(server.port, "list_sets", json!({}));
    assert!(
        !failed,
        "an empty store was reported as a failed call: {said}"
    );
    assert!(
        said.contains("no sets at all") && said.contains("save_set"),
        "an empty store does not say what it is or where sets come from: {said}"
    );

    let hash = stored(&server, PROBE_L1, true);
    set_of(&server, "keeper", &[(Layer::L1, 0, Some("shell"), hash)]);
    let (failed, said) = call(
        server.port,
        "list_sets",
        json!({"holds":"nothing_like_this"}),
    );
    assert!(
        !failed,
        "a filter that matched nothing was an error: {said}"
    );
    assert!(
        !said.contains("no sets at all"),
        "a filter that matched nothing was answered as an empty store, which sends a \
         reader looking for material that is right there: {said}"
    );
    assert!(
        said.contains("none of the 1 set") && said.contains("The store is not empty"),
        "the no-match answer does not say the library is not empty: {said}"
    );
}

/// **What a node is called is one answer, and every node has one.**
///
/// The three cases are the three candidates, in order: the name this set
/// gave the node, the name its procedure gives itself, and the short hash
/// where there is neither. The last two are the ones worth building a
/// fixture for, because both are *ordinary* states of a working store —
/// `Store::put_artifact` writes no card, and a set saved on another machine
/// names artifacts this store has never had — and a listing that dropped
/// either would be a library with holes in it.
///
/// **And it is checked against `read_set`'s own answer**, which is the
/// point of the derivation being one function: a model that picks a set out
/// of a listing and then reads it must find the node it was told about.
#[test]
fn a_node_is_called_here_what_read_set_calls_it() {
    let server = start(true);
    let carded = stored(&server, PROBE_KNOBS, true);
    let uncarded = stored(&server, PROBE_L1, false);
    let elsewhere = Hash::of(b"stored on another machine");
    set_of(
        &server,
        "mixed",
        &[
            (Layer::L1, 0, Some("shell"), carded),
            (Layer::L1, 1, None, carded),
            (Layer::L2, 0, None, uncarded),
            (Layer::L4, 0, None, elsewhere),
        ],
    );

    let (failed, said) = call(server.port, "list_sets", json!({}));
    assert!(!failed, "{said}");
    for expected in [
        // The set's own name, which beats the card's.
        "L1:0 `shell`",
        // No name in the file, so what the procedure calls itself.
        "L1:1 `probe_knobs`",
        // No card at all: the short hash, and the node is listed.
        &format!("L2:0 `{}`", uncarded.short(12)),
        // Not in this store at all: the same, and still listed.
        &format!("L4:0 `{}`", elsewhere.short(12)),
    ] {
        assert!(
            said.contains(expected),
            "the listing does not name a node `{expected}`: {said}"
        );
    }

    // **The same names, from the tool that reads one set.** Two derivations
    // that agree today are two answers that stop agreeing the day one is
    // edited, and this is the assertion that would notice.
    let (failed, read) = call(server.port, "read_set", json!({"id":"mixed"}));
    assert!(!failed, "{read}");
    for expected in [
        "L1:0 `shell`",
        "L1:1 `probe_knobs`",
        &format!("L2:0 `{}`", uncarded.short(12)),
        &format!("L4:0 `{}`", elsewhere.short(12)),
    ] {
        assert!(
            read.contains(expected),
            "`read_set` calls a node something the listing does not: `{expected}` is \
             not in {read}"
        );
    }
}

/// A store that cannot be opened is an error with the path in it, the way
/// `read_set` reports one — not an empty library.
#[test]
fn a_store_that_cannot_be_opened_is_an_error_naming_the_path() {
    let server = start(true);
    // A file where the store's own root has to be: `Store::open` creates
    // the layout under it and cannot, so the open itself is what fails —
    // and the path is the only thing that tells an operator which store
    // this run was pointed at.
    let root = store_root(&server.dir);
    std::fs::write(&root, b"not a directory").expect("write");
    let (failed, said) = call(server.port, "list_sets", json!({}));
    assert!(
        failed,
        "a store that cannot be read answered as though it held nothing: {said}"
    );
    assert!(
        said.contains(&root.display().to_string()),
        "the failure does not say which store: {said}"
    );
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
        let checked = karakuri_environment::compile::check(source).expect("the fixture compiles");
        store
            .write_meta(&hash, &karakuri_environment::meta::card(&hash, &checked))
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

/// **What a saved set will allocate to hold elements, without building
/// it.**
///
/// `Set::element_storage` has reported this per node since the buffers
/// existed and nothing in this tree printed it; the figure that *was*
/// printed, at stage 4, was a second arithmetic over one procedure's `emit`
/// list and was 85% low. So the test is not that a number appears — it is
/// that the number is the one the allocation is sized by, over material
/// where a per-procedure reading would say something else:
///
/// - **`warp` is charged for `position` and it never mentions it.** An L2
///   writes everything that reached it, so it is sized at the chain's
///   stride; a figure read off its own `deform` block would be another
///   number entirely.
/// - **`dots` has no row at all.** A renderer draws from the buffer the
///   node above it allocated, so a row for it would be the same memory
///   counted twice — and a zero would be a number the reader has to work
///   out the meaning of.
/// - **Sixteen elements and not the eight the procedure defaults to.** The
///   `capacity` record is what this set was saved at, and a figure computed
///   from the declaration instead would be exactly half of every number
///   below while looking just as plausible.
///
/// The bytes are hand-walked from WGSL's placement rules, the same way
/// `karakuri-engine`'s own storage tests are, so nothing here is the engine
/// compared against itself: `emit position` lays out `seed` at 0, then
/// `birth_frac` at 4, then `position` at 16 — 28 bytes rounded up to the
/// struct's 16-byte alignment, so a stride of 32. A geometry keeps two
/// directions of the element buffer and two of the four-byte liveness flag
/// and, having neither `spawn` nor `kill()`, nothing else: `2 * 16 * (32 +
/// 4)` is 1152. The deform keeps one buffer at that stride and no flags of
/// its own: `16 * 32` is 512.
#[test]
fn a_saved_set_says_what_it_will_allocate_to_hold_elements() {
    let server = start(true);
    let store = server.store();
    let put = |source: &str| store.put_artifact(source.as_bytes()).expect("put");
    let (l1, l2, l4) = (put(PROBE_L1), put(PROBE_L2), put(PROBE_L4));
    store
        .write_set(
            "costed",
            &[
                Line::new(Record::Slot {
                    at: karakuri_store::record::NodeAddress {
                        layer: Layer::L1,
                        index: 0,
                    },
                    name: Some("shell".into()),
                    proc_hash: l1,
                }),
                Line::new(Record::Slot {
                    at: karakuri_store::record::NodeAddress {
                        layer: Layer::L2,
                        index: 0,
                    },
                    name: Some("warp".into()),
                    proc_hash: l2,
                }),
                Line::new(Record::Slot {
                    at: karakuri_store::record::NodeAddress {
                        layer: Layer::L4,
                        index: 0,
                    },
                    name: Some("dots".into()),
                    proc_hash: l4,
                }),
                Line::new(Record::Capacity {
                    at: karakuri_store::record::NodeAddress {
                        layer: Layer::L1,
                        index: 0,
                    },
                    value: 16,
                }),
            ],
        )
        .expect("set");

    let (failed, said) = call(server.port, "read_set", json!({"id":"costed"}));
    assert!(!failed, "{said}");
    for expected in [
        "element storage: 1664 bytes in total, across the 2 nodes",
        // **With the indentation, because the assertion below discriminates
        // on it.** A storage row is indented and a node block's head is
        // not; a bare substring here would keep passing on the day the
        // indent went away, and the negative assertion would then be
        // asserting nothing.
        "  `shell` — 1152 bytes for 16 elements, 72 bytes each",
        "  `warp` — 512 bytes for 16 elements, 32 bytes each",
    ] {
        assert!(
            said.contains(expected),
            "the set was not costed as `{expected}`: {said}"
        );
    }
    // **The storage rows are indented and the node blocks are not**, which
    // is what tells the two apart now that a node block names the node in
    // its own head — `L4:0 \`dots\` — stored as …` is the renderer being
    // described, and `  \`dots\` — 512 bytes` would be the renderer being
    // charged for a buffer it does not own.
    assert!(
        !said.contains("  `dots` — "),
        "the renderer was charged for the buffer it draws from, which is the \
         node above it: {said}"
    );
    // **The sentence that keeps this from being read as device memory.**
    // The withdrawn figure's mistake was as much in what it was taken to
    // mean as in its arithmetic, and a number a model relays as "what this
    // costs a GPU" is that mistake in a new costume.
    assert!(
        said.contains("NOT what this set costs a GPU"),
        "an element-storage figure is offered as though it were device \
         memory: {said}"
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

/// **A write lands on the node its address names, and its neighbour is left
/// alone** — asserted on the files rather than on what the call said.
///
/// This is the one property the routing through
/// [`karakuri_operation::Operation`] could quietly lose: the wire's `index`
/// becomes [`NodeAddress::index`] and comes back out again to resolve a file, so
/// an address that arrived correct and was carried wrong would still return
/// *compiled and written* and change the wrong procedure. A slot with two
/// renderers is what makes that visible: with one, every wrong index is the
/// right one.
#[test]
fn a_write_reaches_the_node_its_address_names_and_not_its_neighbour() {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let head = write("l1.kir", PROBE_L1);
    let first = write("l4_a.kir", PROBE_L4);
    let second = write("l4_b.kir", PROBE_L4);
    let reporter = serve(
        0,
        Slots::of(vec![(head, vec![first.clone(), second.clone()])]),
        store_root(&dir),
        true,
        closed(),
    )
    .expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    let (failed, said) = call(
        port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","index":1,"source":PROBE_L4_B}),
    );
    assert!(!failed, "the second renderer could not be written: {said}");
    assert_eq!(
        std::fs::read_to_string(&second).expect("the second renderer"),
        PROBE_L4_B,
        "`L4:1` was addressed and the file behind it does not hold what was written"
    );
    assert_eq!(
        std::fs::read_to_string(&first).expect("the first renderer"),
        PROBE_L4,
        "`L4:1` was addressed and `L4:0` changed — the address did not survive the \
         call, and the answer said the write had landed"
    );
}

// -- what the audit of this file against the engine turned up -----------

/// A slot whose head is anything but a geometry, for the tests below.
///
/// **`--set` takes whatever the operator typed first**, and nothing checks
/// that it is an L1 — the sort that assembles the slot reads every file's
/// own `kind` line, the head included, so a chain headed by a camera is an
/// ordinary slot with an ordinary camera in it.
fn headed_by_a_camera(dir: &tempfile::TempDir) -> (Slots, Vec<std::path::PathBuf>) {
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let camera = write("camera.kir", PROBE_L3);
    let l1 = write("l1.kir", PROBE_L1);
    let l4 = write("l4.kir", PROBE_L4);
    (
        Slots::of(vec![(camera.clone(), vec![l1.clone(), l4.clone()])]),
        vec![camera, l1, l4],
    )
}

/// **The head is addressed under the `kind` it declares**, like every other
/// file of the slot.
///
/// It was filed as `L1:0` whatever it said, which no other reader of these
/// files agrees with: `compile::sort_compiled` matches on the head's own
/// `kind` and `history::seed` reads the head's `kind` line, taking a
/// position only where a file declares nothing. So a slot headed by a
/// camera had its camera at `L1:0`, its geometry unreachable, and its real
/// `L3` addressable by nobody.
#[test]
fn a_head_is_addressed_under_the_kind_it_declares() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (slots, paths) = headed_by_a_camera(&dir);
    let (camera, l1, l4) = (paths[0].clone(), paths[1].clone(), paths[2].clone());

    assert_eq!(
        slots.path(0, Kind::L3, 0).expect("the head is the camera"),
        camera,
        "the head was not filed under the `kind` it declares"
    );
    assert_eq!(
        slots
            .path(0, Kind::L1, 0)
            .expect("the geometry is reachable"),
        l1,
        "`L1:0` did not reach the file that declares `kind L1`"
    );
    assert_eq!(slots.path(0, Kind::L4, 0).expect("the renderer"), l4);

    // The head has taken its own layer's index 0, so a second camera in the
    // chain would be `L3:1` — the counting rule the head always had, now
    // applied on the layer it is actually on.
    let past = slots
        .path(0, Kind::L3, 1)
        .expect_err("this slot was given one camera");
    assert!(past.contains("one L3"), "{past}");
}

/// **A file with no `kind` line at all keeps `history::seed`'s positional
/// answer**: the first path is an L1 and every later one a renderer.
///
/// That fallback is the whole reason reading the head's `kind` is safe. A
/// slot whose head cannot be read — or that is a `.kir` the scan finds no
/// `kind` in — must still be addressable at `L1:0`, because that is where
/// its snapshots are filed.
#[test]
fn a_head_that_declares_nothing_is_still_the_slots_l1() {
    let dir = tempfile::tempdir().expect("tempdir");
    let silent = dir.path().join("silent.kir");
    std::fs::write(&silent, "proc nothing_declared {\n}\n").expect("fixture");
    let mute = dir.path().join("mute.kir");
    std::fs::write(&mute, "proc also_nothing {\n}\n").expect("fixture");
    let slots = Slots::of(vec![(silent.clone(), vec![mute.clone()])]);

    assert_eq!(
        slots.path(0, Kind::L1, 0).expect("the head is the L1"),
        silent
    );
    assert_eq!(
        slots
            .path(0, Kind::L4, 0)
            .expect("a later one is a renderer"),
        mute
    );
}

/// **A write lands on the file its address names, and a camera-headed slot
/// does not lose its camera to a valid L1.**
///
/// This is what the address bug cost: `write_procedure(slot, "L1", 0, …)`
/// with a real geometry in it passed the kind guard — the source said L1
/// and the address said L1 — and overwrote the camera's file. The rebuild
/// then sorted by `kind`, so the slot quietly gained a second geometry and
/// lost the camera it was looking through.
#[test]
fn a_write_addressed_to_a_geometry_does_not_overwrite_the_head() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (slots, paths) = headed_by_a_camera(&dir);
    let (camera, l1) = (paths[0].clone(), paths[1].clone());
    let reporter = serve(0, slots, store_root(&dir), true, closed()).expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    let (failed, said) = call(
        port,
        "write_procedure",
        json!({"slot":0,"layer":"L1","source":PROBE_L1_B}),
    );
    assert!(!failed, "{said}");
    assert_eq!(
        std::fs::read_to_string(&camera).expect("the camera is still there"),
        PROBE_L3,
        "a write addressed to `L1:0` landed on the head, which is the camera"
    );
    assert_eq!(
        std::fs::read_to_string(&l1).expect("the geometry"),
        PROBE_L1_B,
        "the write did not reach the file that declares `kind L1`"
    );

    // And the camera is readable at the address it is on, which is the
    // other half of the same defect: the real L3 could be reached by
    // nobody.
    let (failed, read) = call(port, "read_procedure", json!({"slot":0,"layer":"L3"}));
    assert!(!failed, "{read}");
    assert!(read.contains("probe_camera"), "{read}");
}

/// **An L2 that declares `uses shape : Field` is written cleanly**, which
/// is the promise `write_procedure`'s description had to stop making.
///
/// `compile::check` checks one procedure in isolation; everything between
/// nodes is `Set::validate`, which is where an unbound slot is refused. So
/// a clean write is not a slot that rebuilds, and this is still true after
/// `wire_input` exists: the binding is a **second** call, and the window
/// between the two is a slot that does not build. What changed is that
/// there is now a second call to make — see
/// [`a_uses_written_here_can_be_bound_here_and_the_slot_builds`], which
/// takes this write the rest of the way.
#[test]
fn a_write_that_needs_an_edge_still_returns_cleanly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("fixture");
        path
    };
    let l1 = write("l1.kir", PROBE_L1);
    let warp = write("warp.kir", PROBE_L2);
    let l4 = write("l4.kir", PROBE_L4);
    let reporter = serve(
        0,
        Slots::of(vec![(l1, vec![warp, l4])]),
        store_root(&dir),
        true,
        closed(),
    )
    .expect("serve");
    let port = reporter.port();
    stand_in(reporter, no_loop);

    let (failed, said) = call(
        port,
        "write_procedure",
        json!({"slot":0,"layer":"L2","source":PROBE_L2_USES}),
    );
    assert!(
        !failed,
        "the per-procedure check refused this, so the description's caveat is about \
         something that cannot happen any more: {said}"
    );
}

/// **A run with `--mcp` and no `--watch` has a scratch and an edit history
/// like any other**, and the answer used to tell it the opposite.
///
/// `main.rs`'s `editable` is `watch || mcp.is_some()`: the deck runs from
/// copies, so the files the operator named are never written to, and
/// `history::seed` has filed the version the run started with. "It replaced
/// the file on disk and there is no backup" was wrong in both halves, and
/// it was wrong on the one surface whose reader cannot look at the
/// terminal.
#[test]
fn a_write_without_watch_still_says_where_the_old_version_went() {
    let (server, reporter) = started(false);
    stand_in(reporter, no_loop);
    let (failed, said) = call(
        server.port,
        "write_procedure",
        json!({"slot":0,"layer":"L4","source":PROBE_L4}),
    );
    assert!(!failed, "{said}");
    assert!(
        said.contains("`<store>/history/`"),
        "the version it replaced is in the edit history and this does not say so: {said}"
    );
    assert!(
        said.contains("scratch"),
        "the file it replaced is the run's copy and this does not say so: {said}"
    );
    assert!(!said.contains("no backup"), "there is a backup: {said}");
    // The thing that *is* true of this run stays said: nothing will pick
    // the write up.
    assert!(said.contains("without `--watch`"), "{said}");
}

// -- the edge, which is the other half of a `uses` ---------------------

/// **A stand-in loop that takes the edges as well, and keeps them where a
/// test can look at them.**
///
/// [`stand_in`] answers saves and nothing else, which is what every test
/// before this one needed. What a rewiring *is* belongs to the render loop
/// and needs a deck and a watcher; what these tests are about is that the
/// edge crosses with both its ends intact and that the edge which crosses is
/// one that makes the slot build.
fn wiring_loop(
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

/// A slot of a geometry, a deformation, a renderer and a field — the
/// smallest chain in which a `uses shape : Field` has something to be bound
/// to.
#[allow(clippy::type_complexity)]
fn wired(
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
    )
    .expect("serve");
    let port = reporter.port();
    let seen = wiring_loop(reporter);
    (Server { port, dir }, seen)
}

/// One of the fixture's files, checked, exactly as this surface checks a
/// written one.
fn checked(dir: &std::path::Path, name: &str) -> Checked {
    let path = dir.join(name);
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    karakuri_environment::compile::check(&source)
        .unwrap_or_else(|e| panic!("{} does not check: {e}", path.display()))
}

/// **Whether the fixture's slot assembles**, with the wiring it is given.
///
/// `Set::validate` is `Set::build_many`'s whole check pass and needs no
/// device, which is the only reason this can be asserted here at all: it is
/// the same rule the render loop's rebuild would meet, run against the files
/// that are actually on disk after a write.
fn assembles(
    dir: &std::path::Path,
    edges: &[karakuri_engine::set::Edge],
) -> Result<(), karakuri_engine::set::SetError> {
    let l1 = checked(dir, "l1.kir");
    let warp = checked(dir, "warp.kir");
    let l4 = checked(dir, "l4.kir");
    let blob = checked(dir, "blob.kir");
    karakuri_engine::Set::validate(
        &[(&l1, 8)],
        &[&warp],
        &[],
        &[&blob],
        &[&l4],
        karakuri_engine::set::Layering::Overdraw,
        7,
        &[],
        karakuri_engine::set::Wiring {
            edges,
            ..Default::default()
        },
    )
    .map(|_| ())
}

/// **The test that would have caught it**: a `uses` written through this
/// server, an edge written through this server, and the slot assembling.
///
/// This is the whole trap and the whole fix in one run. `write_procedure`
/// accepted a procedure declaring `uses shape : Field` — it compiles, and
/// one procedure is all `compile::check` ever sees — and the slot then
/// failed to build with `SetError::SlotUnbound`, which nothing on this
/// surface could answer. The middle assertion is that failure, asserted
/// rather than described, so that this test is about a trap that was real;
/// the last is that the edge **this server sent to the loop** is the one
/// that closes it.
#[test]
fn a_uses_written_here_can_be_bound_here_and_the_slot_builds() {
    let (server, seen) = wired(true);
    let dir = server.dir.path().to_path_buf();

    // Before anything: the fixture assembles, so a refusal below is about
    // what the test did and not about the fixture.
    assembles(&dir, &[]).expect("the fixture's own slot assembles");

    let (failed, said) = call(
        server.port,
        "write_procedure",
        json!({"slot":0,"layer":"L2","source":PROBE_L2_USES}),
    );
    assert!(!failed, "{said}");

    // **The trap, on the frame it goes wrong.** The file on disk is the one
    // the model wrote, it checks, and the slot it is in does not assemble.
    let refused = assembles(&dir, &[]).expect_err(
        "a `uses` nothing binds assembled — this test's middle is gone and the two \
         halves either side of it are about nothing",
    );
    assert!(
        matches!(refused, karakuri_engine::set::SetError::SlotUnbound { .. }),
        "the slot failed to assemble for something other than the unbound `uses` this \
         test is about: {refused}"
    );

    // **The way out, over the wire.** Both ends by name: the node is what
    // the procedure calls itself, because nothing named it.
    let (failed, said) = call(
        server.port,
        "wire_input",
        json!({"slot":0,"node":"probe_warp_uses","input":"shape","to":"probe_blob"}),
    );
    assert!(!failed, "{said}");

    let edges: Vec<karakuri_engine::set::Edge> = seen
        .lock()
        .expect("seen")
        .iter()
        .map(|(_, edge)| edge.clone())
        .collect();
    assert_eq!(
        edges.len(),
        1,
        "one call, one edge on the loop's channel — {edges:?}"
    );
    assembles(&dir, &edges).expect(
        "the edge this server sent the render loop does not bind the `uses` this \
         server wrote: a model can still put a slot in a state only the command line \
         gets it out of",
    );
}

/// **Both ends reach the loop as the names that were typed**, and the input
/// is not one of them.
///
/// Three strings on one request is three chances to hand the loop the wrong
/// one, and every mistake of that kind reads as a working call: the edge is
/// written, the slot refuses to build, and the refusal is about a node
/// nobody named. Every one of these three names is a different word, on
/// purpose.
#[test]
fn an_edge_reaches_the_loop_with_the_deck_and_both_ends_as_they_were_typed() {
    let (server, seen) = wired(true);
    let (failed, said) = call(
        server.port,
        "wire_input",
        json!({"slot":0,"node":"declaring_node","input":"the_input","to":"far_end"}),
    );
    assert!(!failed, "{said}");
    let seen = seen.lock().expect("seen");
    let (slot, edge) = seen.first().expect("the loop was sent an edge");
    assert_eq!(*slot, 0, "the deck slot the call named");
    assert_eq!(edge.node, "declaring_node", "the node that declares it");
    assert_eq!(
        edge.slot,
        "the_input".into(),
        "`input` on the wire is the edge's `slot`, which is what the procedure calls \
         its declared input — the deck's slot is the request's own field"
    );
    assert_eq!(edge.to, "far_end", "the node it is bound to");
    // The loop's own words come back to the client unchanged, which is what
    // makes a refusal from the Set legible to a model.
    assert!(
        said.contains("declaring_node") && said.contains("far_end"),
        "what the loop said did not reach the client: {said}"
    );
}

/// **Every part of an edge names something**, which is `parse_edge`'s rule
/// on the command line and the same sentence here.
///
/// An absent part and an empty one are the two shapes, and neither may reach
/// the render loop: an edge with a hole in it is a statement about a node,
/// and there is no such statement.
#[test]
fn every_part_of_an_edge_names_something() {
    let (server, seen) = wired(true);
    for (missing, args) in [
        ("node", json!({"slot":0,"input":"shape","to":"probe_blob"})),
        (
            "input",
            json!({"slot":0,"node":"probe_warp","to":"probe_blob"}),
        ),
        ("to", json!({"slot":0,"node":"probe_warp","input":"shape"})),
        (
            "node",
            json!({"slot":0,"node":"","input":"shape","to":"probe_blob"}),
        ),
        (
            "input",
            json!({"slot":0,"node":"probe_warp","input":"","to":"probe_blob"}),
        ),
        (
            "to",
            json!({"slot":0,"node":"probe_warp","input":"shape","to":""}),
        ),
    ] {
        let (failed, said) = call(server.port, "wire_input", args.clone());
        assert!(failed, "`{args}` was accepted as an edge: {said}");
        assert!(
            said.contains(missing),
            "an edge missing `{missing}` was refused without naming it: {said}"
        );
    }
    // A slot number is still a slot number, and the deck is checked after
    // the four arguments have been read.
    let (failed, said) = call(
        server.port,
        "wire_input",
        json!({"slot":9,"node":"probe_warp","input":"shape","to":"probe_blob"}),
    );
    assert!(failed, "slot 9 was accepted: {said}");
    assert!(
        said.contains("this deck holds"),
        "a slot this deck does not hold was refused in some other surface's words: \
         {said}"
    );
    assert!(
        seen.lock().expect("seen").is_empty(),
        "a refused edge reached the render loop"
    );
}

/// **A run without `--watch` has no watcher, and the answer says so** — the
/// same fact `write_procedure` states about the other half of one edit.
///
/// The edge is still sent: it is the run's wiring from then on, and a
/// `save_set` of that slot records it. What does not happen is the rebuild,
/// and a model told "the slot is rebuilding" by a run that has nothing to
/// rebuild it with would go looking for a change on screen that is never
/// coming.
#[test]
fn an_edge_written_without_watch_says_no_watcher_will_rebuild_the_slot() {
    let (server, seen) = wired(false);
    let (failed, said) = call(
        server.port,
        "wire_input",
        json!({"slot":0,"node":"probe_warp","input":"shape","to":"probe_blob"}),
    );
    assert!(!failed, "{said}");
    assert!(
        said.contains("without `--watch`"),
        "a run with no watcher did not say so: {said}"
    );
    assert_eq!(
        seen.lock().expect("seen").len(),
        1,
        "the edge was not sent at all, so a save of this slot would not record it"
    );

    // And with a watcher, the caveat is absent rather than always printed —
    // a warning that fires on healthy material teaches a reader to skip it.
    let (server, _seen) = wired(true);
    let (failed, said) = call(
        server.port,
        "wire_input",
        json!({"slot":0,"node":"probe_warp","input":"shape","to":"probe_blob"}),
    );
    assert!(!failed, "{said}");
    assert!(
        !said.contains("without `--watch`"),
        "a run that does rebuild was told it does not: {said}"
    );
}

/// **The seven tools still work with every class closed**, over the socket
/// a model actually reaches them on.
///
/// ADR-0235 puts all seven in the open set and promises *"no code in this
/// workspace changes on the day this is recorded"* of their behaviour. The
/// gate is new code on the path every one of them takes, so this is checked
/// rather than assumed — and checked here rather than only over
/// [`asked`], because the gate could have been wired into the wrong seam
/// and a unit test on the right one would never notice.
///
/// **An `operate` the audit passes reaches the render loop's drain and is
/// performed there; one it refuses never reaches it at all.**
///
/// This is the whole claim of the eighth tool, and both halves of it are
/// here because they are one mechanism: the operation is named on a
/// connection thread, audited on that thread, and *performed* on the frame
/// the panel performs a press on — so nothing that a closed class would have
/// stopped can be sitting in the queue when the operator looks
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
///
/// **No device, and a stand-in for the loop.** What a put-back *is* belongs
/// to `crates/karakuri`'s own performer and needs a window; what is asserted
/// here is that the operation crosses the channel with its payload as it was
/// spelled, and that what the loop says is what the client is handed.
///
/// **Watched to fail** with the audit moved after the send: the second call
/// then comes back as a success and the loop has two operations rather than
/// one, which is the failure this is really about.
#[test]
fn an_operate_the_audit_passes_reaches_the_loop_and_a_closed_one_does_not() {
    let (server, reporter) = started(true);
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let taken = seen.clone();
    // The stand-in drains what `Keeping::operated` drains, answers where it
    // answers, and holds the reporter alive — see [`stand_in`], whose shape
    // this is for the third channel.
    std::thread::spawn(move || loop {
        for OperateRequest { operation, reply } in reporter.operations() {
            taken.lock().expect("lock").push(operation.clone());
            reply.settled(Ok(format!("`{}` was performed", operation.title())));
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    });

    let (failed, said) = call(
        server.port,
        "operate",
        json!({
            "operation": "Put a node's previous version back",
            "with": {"deck": 0, "revision": {"picked": "20260908-143052-271_slot0_L4"}},
        }),
    );
    assert!(!failed, "{said}");
    assert!(said.contains("was performed"), "{said}");
    assert_eq!(
        seen.lock().expect("lock").as_slice(),
        [Operation::RestoreProcedure {
            deck: 0,
            revision: karakuri_operation::Revision::Picked("20260908-143052-271_slot0_L4".into()),
        }],
        "the operation reached the loop as something other than what was spelled"
    );

    // And the mix is closed on this fixture, so this one is answered on the
    // connection thread and the loop never hears about it.
    let (failed, said) = call(
        server.port,
        "operate",
        json!({"operation": "Gain", "with": {"deck": 0, "gain": 0.25}}),
    );
    assert!(failed, "{said}");
    assert!(said.contains("the mix faders"), "{said}");
    assert!(said.contains("Mixer"), "{said}");
    assert_eq!(
        seen.lock().expect("lock").len(),
        1,
        "a refused operation reached the render loop"
    );
}

/// **The five ADR-0334 left `plan` reach the loop now, and the one it
/// listed beside them never will.**
///
/// ADR-0334 named six rows that `operate` refused for want of a performer
/// on the frame the drain lands on, and ADR-0341 is the day each of them
/// got one — four by the drain calling the window's own press arm, one by
/// a reading `crates/karakuri` was not taking. **What is asserted here is
/// the half this crate owns**: the name is taken, the payload crosses the
/// channel as it was spelled, and the audit is what stands between them
/// rather than a refusal written in this file. What each of them then *does*
/// needs a window and is asserted where the window is.
///
/// **With every class open**, because four of the five are closed rows and
/// a fixture that left them shut would assert the gate a second time
/// instead of the route. The one that is open either way is the star: its
/// class is nobody's, the refusal it meets is the performer's, and
/// `gate.rs` is untouched (ADR-0301).
///
/// **No count in the name.** It was five when it was written and six by
/// the end of the day, because ADR-0338's two moved in another session
/// while this one was running — so the list below is what it is and the
/// name does not have to be edited when it grows again.
///
/// **Watched to fail** with any of them taken out of `sayable`'s operable
/// list: the call is refused on the connection thread, `failed` is true,
/// and the loop sees nothing.
#[test]
fn the_rows_that_grew_a_performer_reach_the_loop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let l1 = dir.path().join("l1.kir");
    let l4 = dir.path().join("l4.kir");
    std::fs::write(&l1, PROBE_L1).expect("l1");
    std::fs::write(&l4, PROBE_L4).expect("l4");
    let opening = karakuri_environment::Opening::closed();
    opening.set(
        karakuri_operation::gate::Class::ALL
            .iter()
            .fold(karakuri_operation::gate::Open::CLOSED, |open, class| {
                open.with(*class, true)
            }),
    );
    let reporter = serve(
        0,
        Slots::of(vec![(l1, vec![l4])]),
        store_root(&dir),
        true,
        opening,
    )
    .expect("serve");
    let port = reporter.port();

    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let taken = seen.clone();
    std::thread::spawn(move || loop {
        for OperateRequest { operation, reply } in reporter.operations() {
            taken.lock().expect("lock").push(operation.clone());
            reply.settled(Ok(format!("`{}` was performed", operation.title())));
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    });

    // The five, each with the call written beside its row in [`SPELLED`],
    // so this test and the schema a client reads cannot come apart.
    let asked = [
        (
            "Set a deck's mask position",
            json!({"deck": 0, "position": 0.5}),
            Operation::SetMaskPosition {
                deck: 0,
                position: 0.5,
            },
        ),
        (
            "Narrow the published interface",
            json!({"deck": 0, "controls": []}),
            Operation::Publish {
                deck: 0,
                controls: Vec::new(),
            },
        ),
        (
            "Star a Set, or take the star off",
            json!({"set": "a_set", "favourite": true}),
            Operation::SetFavourite {
                id: "a_set".to_string(),
                favourite: true,
            },
        ),
        (
            "Choose where the frame goes",
            json!({"output": "projector", "index": 0, "on": true}),
            Operation::RouteFrame {
                output: karakuri_operation::Output::Projector(0),
                on: true,
            },
        ),
        (
            "Record the session",
            json!({"recording": "start"}),
            Operation::RecordSession {
                recording: karakuri_operation::Recording::Start { id: None },
            },
        ),
    ];
    for (title, with, expected) in &asked {
        let (failed, said) = call(port, "operate", json!({"operation": title, "with": with}));
        assert!(
            !failed,
            "`{title}` is refused over the wire with every class open: {said}"
        );
        assert!(
            said.contains("was performed"),
            "`{title}` was answered by something other than the loop: {said}"
        );
        assert!(
            seen.lock().expect("lock").contains(expected),
            "`{title}` reached the loop as something other than what was spelled — {:?}",
            seen.lock().expect("lock")
        );
    }

    // **And ADR-0338's load, which is taken here and answered by the
    // audit rather than by this file.** Its class is a predicate over the
    // deck it names and this server has read no residency, so it is
    // refused with the reading nobody took — `LoadSet`'s own answer beside
    // it (ADR-0334), and a built route rather than a missing one. What is
    // asserted is *which* refusal: a spelling that did not take the name
    // would refuse it before the gate ever saw it.
    let (failed, said) = call(
        port,
        "operate",
        json!({
            "operation": "Load a procedure over a layer",
            "with": {"deck": 0, "procedure": "orbit_wide"},
        }),
    );
    assert!(
        failed,
        "the load was accepted with no residency read: {said}"
    );
    assert!(
        !said.contains("no operation `") && !said.contains("no route on this surface"),
        "the load was refused by this file's spelling rather than by the audit: {said}"
    );

    // **And the send is refused, in a sentence naming the flag that
    // sends.** It is the row that is `gap` rather than `plan`: a send names
    // no destination and a model cannot answer the dialog the panel puts
    // one in, and a take names a file, which never crosses this protocol.
    let (failed, said) = call(
        port,
        "operate",
        json!({"operation": "Send a Set to somebody, and take one in", "with": {"set": "a_set"}}),
    );
    assert!(failed, "the send was accepted: {said}");
    assert!(
        said.contains("--package") && said.contains("--take-in"),
        "the send's refusal does not say where a model's operator sends one from: {said}"
    );
    assert_eq!(
        seen.lock().expect("lock").len(),
        asked.len(),
        "a refused operation reached the render loop"
    );
}

/// **Two assertions, and the weaker one covers more.** The four this
/// fixture can carry to a real answer must succeed outright. All seven must
/// come back saying something other than the refusal — a tool that fails
/// because this fixture has no store, no saved set and no render loop is
/// this fixture failing it, and a tool the audit stopped says so in the one
/// sentence, which is what makes the two distinguishable at all.
#[test]
fn the_seven_tools_still_work_with_every_class_closed() {
    let server = start(true);
    let asked = [
        ("read_procedure", json!({"slot": 0, "layer": "L4"}), true),
        (
            "write_procedure",
            json!({"slot": 0, "layer": "L4", "source": PROBE_L4}),
            true,
        ),
        ("swap_outcome", json!({}), true),
        ("list_sets", json!({}), true),
        // Answered by this fixture's refusals rather than by the gate: no
        // set was ever saved, and `no_loop` is a render loop that says so.
        ("read_set", json!({"id": "never_saved"}), false),
        ("save_set", json!({"slot": 0}), false),
    ];
    for (name, args, must_succeed) in asked {
        let (failed, said) = call(server.port, name, args);
        assert!(
            !said.contains("closed by default"),
            "`{name}` was stopped by the audit, and ADR-0235 puts all seven tools in the \
             open set: {said}"
        );
        if must_succeed {
            assert!(!failed, "`{name}`: {said}");
        }
    }

    // **The seventh needs the other half of the surface**, so it gets the
    // fixture that has one: `no_loop` never applies an edge, and a client
    // waiting out `WIRE_REPLY` for it would be this test hanging rather
    // than this test failing.
    let (wiring, _seen) = wired(true);
    let (failed, said) = call(
        wiring.port,
        "wire_input",
        json!({"slot": 0, "node": "probe_warp_uses", "input": "shape", "to": "probe_blob"}),
    );
    assert!(!said.contains("closed by default"), "{said}");
    assert!(!failed, "`wire_input`: {said}");
}
