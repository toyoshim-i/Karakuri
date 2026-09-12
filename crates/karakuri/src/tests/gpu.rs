//! The console, through `egui`, through `wgpu` 30, onto a real device.

/// A temporary root nobody else's run wrote into, shared with `mod tests`
/// rather than spelled twice: two answers to *where does a test put its store*
/// is two directories to clean up and one of them stale.
use super::tests::scratch_dir;
use super::*;
/// The engine's own fitting, asked rather than re-derived. It is what
/// `Present::draw` sets its viewport from, so what it leaves over at the edges
/// of a target is exactly the bar that gets cleared to black — and a copy of
/// the arithmetic here would be a test agreeing with itself about the one thing
/// it is checking.
use karakuri_engine::letterbox;

/// What a frame these tests compose advances by.
///
/// A stated count rather than a measured one, and it is honest here for the
/// reason it was not in the live path: these frames are composed to assert what
/// was *drawn* — a cell that took a pass, the rectangle it was aimed at — and
/// they time nothing at all, so this is a fixture rather than a measurement
/// withheld. The live count is `App::clock`, which measures the interval and
/// writes it into the `tick` (ADR-0297).
const STEPS_A_FRAME: u8 = 1;

/// A [`Keeping`] with nothing served and nothing yet built, which is what a run
/// holds on its first frame.
fn keeping() -> Keeping {
    let (_, built) = std::sync::mpsc::channel();
    let (save_tx, saves) = std::sync::mpsc::channel();
    let (send_tx, sends) = std::sync::mpsc::channel();
    let (keep_tx, keeps) = std::sync::mpsc::channel();
    Keeping {
        mcp: None,
        playing: Playing {
            playing: Vec::new(),
        },
        built,
        pending: Vec::new(),
        saves,
        save_tx,
        sends,
        send_tx,
        keeps,
        keep_tx,
        in_flight: 0,
    }
}

/// One request, one reply, over TCP exactly as a client would — the shape
/// `karakuri-environment/src/mcp.rs`'s own `wire_tests` use, restated here
/// because that module is `#[cfg(test)]` and nothing outside it can call in.
///
/// Over a socket, because that is the only way to read a [`mcp::Reporter`]
/// back. A report handed to the server goes into a queue only the protocol can
/// drain, which is exactly the property under test: a swap the lane drew is a
/// swap a model can ask about.
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
    let reply: serde_json::Value = serde_json::from_slice(&body).expect("the answer is not JSON");
    let result = &reply["result"];
    (
        result["isError"].as_bool().unwrap_or(true),
        result["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    )
}

/// A save writes what the deck is playing as a Set file, and the file loads
/// back.
///
/// This is the whole of what *Keep what a deck is playing* is, from the press
/// or the tool call through to a `.set` in the store — and it is a device test
/// because the one line of it that needs a `Deck` is the one that reads what
/// the slot is playing ([`playing_values`]).
///
/// It is asserted against the deck rather than against the flags, which is the
/// whole reason that function exists: the capacity written down is the
/// geometry's own declaration and the salt is the one this slot is running at,
/// so a save that read the command line would record a picture nobody has seen.
///
/// Nothing has been rebuilt when this saves, which is the case
/// [`Playing::at_launch`] exists for: a deck could not be written down at all
/// until the launch version had an address, and the failure it replaces is a
/// refusal saying the sources are not in the store on a run where they are.
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
        None,
        mcp::Slots::unpointed(),
    );

    let root = scratch_dir("kept");
    let mut keeping = keeping();
    keeping.playing = Playing::at_launch(&engine.placed, engine.deck.slot_count());
    // Deck B, so a hard-coded slot 0 fails here.
    keeping.save_set(
        &engine,
        &root,
        Asked::Operator,
        ASKED_TO_PRIME,
        Some("kept01".into()),
        None,
    );
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

/// The swap report says what the lane says, because the lane and the
/// server are told by one drain.
///
/// `Deck::events` empties the channel, so there is no second drain to be
/// had: a loop that read it again would read nothing, and a server told
/// from anywhere else would be told about a different build. That is why
/// [`staging`] reports rather than a function beside it, and this is the
/// check that fact owes — the sentence a model reads out of `swap_outcome`
/// is the event the row was written from.
///
/// # The swap and its verdict arrive together, so the row may already be
/// gone
///
/// This asserted `Stage::Landed` on a row that is no longer there, and
/// it was written when a candidate went on trial: the swap landed in one
/// drain and the verdict came thirty-eight frames later in another, so
/// between them the lane held a `landed` row. ADR-0313 put the verdict in
/// the call the swap lands in, so one `staging` pass sees `Swapped` and
/// its verdict, and a build that held the budget has its row settled in the
/// same pass that made it — *"empty is this lane's ordinary state"*.
///
/// So what this pins is the pair, on whichever verdict this machine's
/// clock produces, which is `docs/contributing.md` §1's rule: a test that
/// turns on whether this adapter is fast is not a test. One frame of the
/// shipped example against a 20 ms budget is comfortable on the machines
/// this was written on and is not a fact about every machine, and the two
/// outcomes are two states of the instrument rather than a pass and a
/// failure:
///
/// - it ran — no row, the capsule on `landed`, no cell marked, and the
///   server's sentence saying it held the budget;
/// - it was stopped — a row on `overloaded`, the capsule with it, that
///   deck's cell marked as a still, and the server's sentence saying so
///   (ADR-0316).
///
/// Which of the two happened is read off the deck (`Deck::overloaded`) and
/// never off the surfaces being checked, or this would be asserting that
/// three readings of one value agree with themselves.
///
/// And what the slot is now playing moves with it. A build that landed
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
    // **One handle for the deck and the server**, which is [`main`]'s
    // arrangement and not a second one: the engine's watchers publish into
    // it and the server resolves through it.
    let pointing = mcp::Slots::of(
        running
            .iter()
            .map(|pair| (pair.l1.clone(), vec![pair.l4.clone()]))
            .collect(),
    );
    let mut engine = Engine::new(
        &gpu,
        &mut renderer,
        &running,
        panel.layout(),
        1.0,
        Some((std::sync::Arc::clone(&store), built_tx)),
        None,
        pointing.clone(),
    );

    let reporter =
        mcp::serve(0, pointing, root.clone(), true, Opening::closed()).expect("an ephemeral port");
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
    // The transport's health capsule, off the same drain, so this test
    // reads both halves of what one verdict writes.
    let mut health: Option<view::Stage> = None;
    // **`staging` answers whether a build landed**, and since ADR-0326 it
    // takes the build up as well — the diff that makes the rows is the
    // same read that replaces what the slot is playing, so there is no
    // second pass here to do it in.
    let mut landed = false;
    let deadline = Instant::now() + Duration::from_secs(30);
    while !landed && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
        drop(engine.deck.begin_frame(&gpu.device, &gpu.queue));
        landed |= staging(
            &mut engine.deck,
            &mut keeping,
            &engine.aimed,
            &mut rows,
            &mut health,
        );
    }
    assert!(
        landed,
        "nothing was built in 30s — the watcher never saw the edit"
    );
    // **What the verdict was, read off the deck** — not off any of the
    // three surfaces below, which is what makes them a check rather than a
    // value compared with itself. See this test's documentation for why
    // both answers are states of the instrument and neither is a failure.
    let stopped = engine.deck.overloaded(EngineSlot(ON_AIR as u8));
    let row = rows.iter().find(|row| row.deck == ON_AIR);

    // **The same sentence, out of the server.** `swap_outcome` answers with
    // the reports the render loop handed over, newest last. Read before the
    // surfaces are checked, because every one of them is checked against
    // it.
    let (failed, said) = call(port, "swap_outcome", serde_json::json!({}));
    assert!(!failed, "swap_outcome refused: {said}");
    assert!(
        said.contains(&format!("slot {ON_AIR}:")),
        "the server was told about a slot this test did not rebuild: {said}"
    );
    // The swap itself, which happened either way and is what `took` above
    // is an entry of.
    assert!(
        said.contains("swapped in"),
        "the swap reached the lane and did not reach the server: {said}"
    );

    // **The transport's health capsule, which is the same drain's other
    // reader.** A `swap::Event` cannot be made without a device, so this is
    // where *the window writes down what the last write did* is checked at
    // all: `karakuri-console` can be asked whether a capsule draws the
    // verdict it was handed, and nothing in that crate can be asked whether
    // this program hands it one. A version that drew the capsule perfectly
    // and never filled `Readout::health` would leave every console test
    // green — which is the seam `mod press_handler` exists for, on the
    // readout's side, where the scan it uses cannot see anything at all.
    match stopped {
        // **It ran.** The verdict was in the candidate's favour, so the
        // file and the picture agree and the row left the lane in the same
        // pass that made it — `view::staging`'s own rule, and the page's
        // *empty is this lane's ordinary state*. The capsule keeps the
        // swap's word, because a verdict in favour is not a write.
        false => {
            assert!(
                row.is_none(),
                "the build held the budget and its row is still on the lane, \
                 which is a lane that never empties: {:?}",
                row.map(|row| row.stage)
            );
            assert_eq!(
                health,
                Some(view::Stage::Landed),
                "the lane was told what the build did and the transport row was not"
            );
            assert!(
                said.contains("held the budget") || said.contains("was not judged"),
                "the lane settled this build and the server was told something \
                 else about it: {said}"
            );
        }
        // **It was stopped** (ADR-0316). The version is in the slot, the
        // slot is not stepping it, and three surfaces say so in one word.
        true => {
            let row = row.expect(
                "the slot was stopped for cost and the lane drew no row, which is \
                 the disagreement this lane exists to say",
            );
            assert_eq!(
                row.stage,
                view::Stage::Overloaded,
                "the slot is stopped and its row says otherwise"
            );
            assert_eq!(
                health,
                Some(view::Stage::Overloaded),
                "the lane says the slot is stopped and the transport row does not"
            );
            assert!(
                said.contains("is overloaded"),
                "the lane says the slot is stopped and the server was told \
                 something else: {said}"
            );
            assert!(
                said.contains(&row.name),
                "the server was told about a build the row was not: {said} \
                 against `{}`",
                row.name
            );
        }
    }

    // **And the cells' half of the same reading**, which is the other thing
    // a verdict writes into the view: only the slot the verdict was against
    // is drawn as a still, and the entries past this deck's slot count are
    // `false` rather than a panic, because the row is `view::DECKS` cells
    // whatever the deck holds.
    let mut cells = [false; view::DECKS];
    cells[ON_AIR] = stopped;
    assert_eq!(
        stopped_slots(&engine.deck),
        cells,
        "the cells do not say what the deck says about which slot is stopped"
    );

    // And the slot is playing the new bytes, so a save would write them —
    // taken up inside `staging`, where the diff that made the rows above
    // was taken.
    let now = keeping.playing.at(ON_AIR).expect("still addressable")[0].hash;
    assert_ne!(
        was, now,
        "the build landed and the slot is still addressed as what it launched with"
    );
    // **And the row names the node the edit was of.** The edit was to the
    // slot's L1 and to nothing else, so a build that reported the whole
    // stack and a diff that compared it against what was playing come to
    // one changed node — which is the whole of ADR-0326 asserted against a
    // real watcher rather than against a `Changed` this file built.
    if let Some(row) = rows.iter().find(|row| row.deck == ON_AIR) {
        assert_eq!(
            row.addr, "L1:0",
            "the edit was to the L1 and the row says `{}`",
            row.addr
        );
        assert_eq!(
            rows.iter().filter(|row| row.deck == ON_AIR).count(),
            1,
            "one file changed and the lane drew a row for every node of the stack"
        );
    }

    std::fs::remove_dir_all(&root).expect("clean up");
}

/// A real Set reads out into a pane, which is the seven reads [`inspector`]
/// makes held against a Set this program actually builds rather than against a
/// fixture it wrote itself (`docs/contributing.md` §3 — the criterion is not
/// that it probably will not change but that it *can*, and this window's input
/// is the product).
///
/// It is the one place the resolution in [`node_of`] is checked end to end:
/// this deck's Sets have the default interface, so every control an author
/// could have addressed is a wildcard, and if the resolution were wrong the bay
/// would draw node heads with nothing under them and every other test would
/// still pass.
///
/// Three of them are addressed all the same, and they are the built-in camera's
/// — `radius`, `speed` and `height`, which the engine declares for a node with
/// no procedure behind it and publishes with their address because a bare name
/// does not reach them
/// (`docs/adr/0318-the-built-in-cameras-three-placement-numbers-are-parameter-rows.md`).
/// That is what makes the last assertion here worth more than it was: the pair
/// this deck opens on declares a `radius` of its own, so the two halves of
/// `node_of` are both exercised on one key — the wildcard one landing on the L1
/// alone, and the addressed one landing on the camera — and a resolution that
/// counted the camera as a second declaration would drop the geometry's row and
/// leave the count short. It did, until the landing stopped being worked out
/// here.
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
        None,
        mcp::Slots::unpointed(),
    );
    // One name per slot, which is what `Gfx::material` is: every slot
    // opens on the same pair, and a load is what makes them differ.
    let material = vec![sources.material(); engine.deck.slot_count()];
    let mut panes = Vec::new();
    // **The run's own aims**, so the `uses` lines are read off what each
    // slot is actually pointed at rather than off an empty list. The pair
    // this panel opens on declares no input, so every group's line count is
    // zero — which is what the assertion below says rather than assumes
    // (ADR-0329).
    inspector(
        &engine.deck,
        &material,
        &engine.aimed,
        view::PANE_DECKS,
        &mut panes,
    );

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

    // **The deck head's two build chips, read off the Set rather than off
    // any number this file chose.** Every assertion below is against what
    // the engine answers, because the numbers belong to
    // `examples/drift_shell.kir` — a file the MCP surface exists to rewrite
    // — and a fixture the product can rewrite is not a fixture
    // (`docs/contributing.md` §3). What is being checked is that the offer
    // is the *material's* declaration: a ladder built from the wrong range
    // is a chip that asks for builds the engine refuses (ADR-0328).
    let set = engine.deck.slot(karakuri_engine::DeckSlot(0)).set();
    let aimed = pane
        .aimed
        .as_ref()
        .expect("a slot with a geometry has a capacity and a salt");
    assert_eq!(
        aimed.capacity,
        set.source_capacities()[0],
        "the chip reads a capacity the first geometry is not running at"
    );
    let declared = set.declared_capacities();
    assert_eq!(
        aimed.stated,
        aimed.capacity != declared[0][2],
        "lit and unlit disagree with whether the slot is on its material's own default"
    );
    assert!(
        !aimed.capacities.is_empty(),
        "one geometry declaring a range gave a chip with nothing to step to"
    );
    assert!(
        aimed.capacities.windows(2).all(|two| two[0] < two[1]),
        "the ladder is not ascending, so the step is not a step: {:?}",
        aimed.capacities
    );
    for rung in &aimed.capacities {
        assert!(
            rung.is_power_of_two(),
            "{rung} is on the ladder and is not a power of two"
        );
        for at in declared {
            assert!(
                (at[0]..=at[1]).contains(rung),
                "the chip offers {rung} and a geometry declares [{}, {}], so the build this \
                 press asks for is one the engine refuses by name",
                at[0],
                at[1]
            );
        }
    }
    // **The running value is inside the declared range and is not
    // necessarily on the ladder**, and the pair this panel opens on is the
    // demonstration rather than a corner: it runs at 10240, which
    // `examples/coil_vortex.kir` declares as its default and which is not a
    // power of two. So the *shipped* state of this program is the one
    // `view::stepped_capacity` steps **up** from — the case a `position`
    // lookup would have answered by dropping the slot to the bottom of its
    // own range.
    assert!(
        (declared[0][0]..=declared[0][1]).contains(&aimed.capacity),
        "the pair runs at {} and its geometry declares [{}, {}]",
        aimed.capacity,
        declared[0][0],
        declared[0][1]
    );
    let above = aimed
        .capacities
        .iter()
        .find(|rung| **rung > aimed.capacity)
        .or_else(|| aimed.capacities.first());
    assert!(
        above.is_some(),
        "a slot running at {} has nowhere to step inside {:?}",
        aimed.capacity,
        aimed.capacities
    );
    // **The next salt in the slot's own sequence, from the salt the slot is
    // running** — the console cannot compute this and must not, so what is
    // checked here is that the number handed over is the engine's own
    // derivation and not the value it was derived from (P-0092).
    let running = set.source_salts()[0];
    assert_eq!(
        aimed.salt,
        karakuri_engine::set::derived_salt(running, 1),
        "the salt offered is not the next in this slot's sequence"
    );
    assert_ne!(
        aimed.salt, running,
        "a press would ask for the salt the slot is already on, which is a rebuild that \
         changes no pixel"
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
            node.authority.map(|a| a.level),
            Some(karakuri_operation::Authority::Manual),
            "{} reads something other than the default nobody has changed",
            node.addr
        );
        // **And the chip carries the node it is about**, because a press
        // on one has to say which node — the address is `Set::node_named`'s
        // and not `Node::addr` read back, which is a display string
        // (ADR-0286).
        assert!(
            node.authority.is_some(),
            "{} draws a chip with no node behind it",
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
    let published = engine
        .deck
        .slot(karakuri_engine::DeckSlot(0))
        .set()
        .published()
        .len();
    assert!(published > 0, "the pair publishes what it declares");
    // **No node of this pair declares an input**, so no group draws a
    // `uses` line — read off the aims rather than assumed, because an empty
    // list here and an empty *reading* are two different facts and only one
    // of them is about the material.
    assert!(
        pane.nodes.iter().all(|node| node.uses.is_empty()),
        "a node of the launch pair drew a `uses` line, and the pair declares no input"
    );

    // **The rows off the interface are not among them**, and on this pair
    // there are none: nobody has narrowed anything, so `Set::published`
    // answers with `declared_interface` itself and every row has a number
    // (ADR-0329).
    let mut ords: Vec<usize> = pane
        .nodes
        .iter()
        .flat_map(|node| node.params.iter().filter_map(|param| param.ord))
        .collect();
    assert_eq!(
        ords.len(),
        pane.nodes
            .iter()
            .map(|node| node.params.len())
            .sum::<usize>(),
        "a row of this pane carries no interface position, and nothing has narrowed this deck"
    );
    ords.sort_unstable();
    assert_eq!(
        ords,
        (1..=published).collect::<Vec<_>>(),
        "a published control lost its group, so a row this Set publishes is not drawn"
    );
}

/// ADR-0155's other half, as an assertion: the engine's texels reach the panel.
///
/// The first half — `egui` and `karakuri-engine` resolving one `wgpu` and
/// sharing one `Device` — is what `egui_paints_the_console_onto_a_device` below
/// settles. This is the question that was left: a Set is built, a deck frame is
/// rendered, the present pass letterboxes the canvas into the picture's
/// rectangle, and the panel's own pass samples that texture — all into one
/// command encoder and one submission, engine first — and what lands in the
/// window is read back.
///
/// # Why the assertion is "lit" and not "not the bay's colour"
///
/// A picture that never had anything drawn into it is black, and black is
/// already not `--c-panel`. So "the picture is not the card" passes for a
/// texture that was registered, sampled and never rendered — which is exactly
/// what the ordering defect produces: record the panel's pass before the
/// engine's and the frame samples an empty texture, with no complaint from
/// anywhere. The particles are the evidence, so the count of lit texels inside
/// the picture is what is asserted, and the two controls beside it — the bay's
/// own body, and the deck preview row — say the picture stayed in its region.
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
        None,
        mcp::Slots::unpointed(),
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
    (view.picture, view.previews) = engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
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
                let user =
                    renderer.update_buffers(&gpu.device, &gpu.queue, encoder, &primitives, &screen);
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

/// The picture's texture is the size of the picture, the picture is the
/// canvas's shape, and the texture therefore carries no bars.
///
/// Three claims and every one of them fails without a mark on the screen. A
/// texture sized from the window looks perfectly correct — the picture fills
/// whatever rectangle it is given — and is wrong by however much the panel is
/// not the picture, which here is most of it. A texture sized from the whole
/// region looks perfectly correct too, and that is the one this change is
/// about: it is the shape of the region rather than of the canvas, so
/// `Present::draw` fills the middle of it and clears the rest, and the bars are
/// allocated, cleared and sampled sixty times a second for nobody. At this
/// window that is 225 texels down each side of a 916-wide texture.
///
/// The bars are asked of the engine's own `letterbox` rather than re-derived
/// here, because that is the function that draws them: it answers where the
/// canvas sits inside the texture, so a bar is what it leaves over. What is
/// asserted is that the bar is under one texel — not zero, and the difference
/// is the whole of why `Present::draw` stays. `picture_rect` rounds to whole
/// pixels, so the picture is the mock's 466 x 262 rather than exactly 16:9, and
/// the fit still has a quarter of a pixel to absorb. Sub-texel is what this
/// change makes it; redundant is what it does not.
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
        None,
        mcp::Slots::unpointed(),
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

/// The frame is composited at the picture's rectangle, and a projector raises
/// it — ADR-0325, on a real device rather than on `render_size`'s arithmetic.
///
/// # Why this is not `render_size`'s test said twice
///
/// `render_size` is a maximum over sizes and is tested on the CPU. What this
/// asserts is the *wiring*: that the picture's size is what reaches that
/// maximum, that both the deck and the present pass followed the answer, and
/// that they followed it together — which is the one thing a caller can get
/// wrong here and be told about a frame later, by `Frame::render`'s size check
/// panicking at the call site. Injecting a resize of one and not the other
/// passes every CPU test in this file.
///
/// The projector's size is handed in rather than a window opened, because no
/// test in this workspace can open one: `ActiveEventLoop::create_window` needs
/// a live event loop and `mod gpu` has none. That is the seam `Engine::aim`'s
/// `projector` argument is on, and it is exactly why the argument is a size
/// rather than a `&Projector`.
#[test]
fn the_frame_is_composited_at_the_largest_enabled_output() {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const W: u32 = 1440;
    const H: u32 = 900;

    let gpu = Gpu::headless().expect("no GPU");
    let mut renderer =
        egui_wgpu::Renderer::new(&gpu.device, FORMAT, egui_wgpu::RendererOptions::default());

    let mut panel = Panel::new(W as f32, H as f32);
    view::rearrange(&mut panel, CANVAS);
    let picture = physical(
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
        None,
        mcp::Slots::unpointed(),
    );

    // **The picture alone**, which is every run this program opens on.
    engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
    assert_eq!(
        engine.present.size(),
        picture,
        "the frame is composited at {:?} while the only output on is the picture at \
         {picture:?} — every output is a downscale of the one render, so a frame larger \
         than its only destination is a render nobody asked for and a frame smaller is \
         an upscale",
        engine.present.size()
    );
    assert_ne!(
        picture, CANVAS,
        "this window's picture happens to be exactly the session canvas, so the \
         assertion above cannot tell the derivation from the constant it replaced"
    );

    // **A projector on, and it is larger**, so the frame follows it and
    // the picture becomes a downscale of one render.
    let projector = (3840, 2160);
    engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, Some(projector));
    assert_eq!(
        engine.present.size(),
        projector,
        "a projector larger than the picture did not raise the frame, so it would be \
         shown an upscale of the picture's size"
    );

    // **And the deck followed the present pass.** Composing is what says
    // so: `Frame::render` checks the size it is handed against the deck's
    // and panics at the call site, so a deck left at the old size is a
    // panic here rather than a black frame later.
    let Engine {
        deck,
        present,
        picture: into,
        look,
        ..
    } = &mut engine;
    let mut sinks: [&mut dyn Sink; 1] = [into];
    compose(
        &gpu,
        deck,
        present,
        &mut sinks,
        &mut |_, _| {},
        |_| Committed {
            steps: STEPS_A_FRAME,
            look: *look,
        },
        |_| {},
    )
    .expect("the frame composes at the derived size");

    // **The projector off again, and the frame comes back down.** The
    // picture is still on, so the maximum is over one output and it is the
    // picture's — which is the half of the rule that costs: turning a
    // larger sink on raises what every frame costs and turning it off
    // lowers it again, visibly and by the operator's own act. The other
    // half, where *every* output is off and the size stands, is
    // `render_size`'s `None` and is asserted on the CPU.
    engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
    assert_eq!(
        engine.present.size(),
        picture,
        "with the projector gone the picture is the only output left and the frame is \
         its size again"
    );
}

/// A wider window remakes no texture at all, and a drag on the program's height
/// remakes one and frees the registration it replaces.
///
/// The first half is new and is the saving this change is for. The picture's
/// rectangle used to follow the window's width, so every frame of a horizontal
/// drag was a texture destroyed and rebuilt and a registration freed and
/// re-registered — on the render thread. It is the canvas's shape now and the
/// arrangement pins its height, so a widening moves nothing and there is
/// nothing to remake. That is asserted rather than described, because a rule
/// that quietly went back to remaking costs exactly what it used to and says
/// nothing.
///
/// The second half is the claim the first one must not be allowed to weaken: a
/// `register_native_texture` with no `free_texture` beside it leaks a bind
/// group and a sampler per remade frame, and the height is still something an
/// operator drags. So the free is asserted on the resize that still happens.
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
        None,
        mcp::Slots::unpointed(),
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
    engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
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
    // A boundary, so there is no destination to hand in — see
    // `Panel::released`, which takes one for the carry's sake alone.
    panel.released(None);
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

    engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
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
    engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
    assert_eq!(engine.freed, 1);
    assert!(renderer.texture(&engine.picture.id).is_some());
}

/// Every cell with a deck slot behind it is aimed, whatever that slot's
/// residency — and a cell with no slot behind it is off.
///
/// This is
/// [ADR-0258](../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md)
/// on this surface. The operator decides whether to raise a fader by watching
/// the cell, so the cell has to be running *before* the fader goes up; a cell
/// gated on `Residency::Live` answers the question only after it has stopped
/// being asked, and that gate was here.
///
/// All four residency arrangements, and the one that fails a constant. A cell
/// aimed because a constant said four would pass this while being the older
/// defect in the other direction — so the deck is put through every level,
/// including all four Allocated, where a live-gated `aim` reports nothing at
/// all and a correct one reports four.
///
/// The empty case is the fourth cell of a deck that does not have one.
/// `Engine::new` says a slot cannot hold nothing — `HotSwap::new` takes a live
/// `Set` — so *empty* is not a slot with no material, it is a cell with no
/// slot: `Deck::slot_view` is `None` past `slot_count`, the bind group is
/// `None`, no pass is recorded, the view's entry stays `None` and
/// `karakuri_console::view` draws `D · no slot` in `pal.faint`. This deck is
/// full, so that is asserted where it can be — the view past the last slot —
/// rather than by building a short deck this program cannot have.
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
        None,
        mcp::Slots::unpointed(),
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
        engine.deck.slot_view(EngineSlot(DECKS as u8)).is_none(),
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
        engine
            .deck
            .set_residency(EngineSlot(slot as u8), Residency::Live);
    }
    engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
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
            engine.deck.set_residency(EngineSlot(slot as u8), residency);
        }
        let (_, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), 1.0, None);
        for (slot, aimed) in previews.into_iter().enumerate() {
            let aimed = aimed.unwrap_or_else(|| {
                panic!(
                    "cell {} was not aimed with the deck at {residencies:?} — a slot's \
                     own material is what an operator watches to decide whether to put \
                     it on air, so a cell that waits for its deck to be Live is dark at \
                     the one moment it is wanted (ADR-0258)",
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

/// Clear a cell's texture, so that what is in it afterwards can only have come
/// from a pass recorded after this one.
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

/// How many texels of a cell's texture are not transparent black. The row pitch
/// is padded to 256 because a cell is 112 wide and `copy_texture_to_buffer`
/// will not take 448.
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

/// Deck A's texture is the size of its cell, a resize frees the
/// registration it replaces, and the tally counts both textures.
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
/// and it widened the window by 400 to prove it. That is false since the
/// bay started arranging itself, and it was false at exactly the two
/// widths this test already used: 1440 is below the crossover and 1840 is
/// past it, so the 400 the test widens by is the one resize that makes a
/// cell eleven times the texels it was.
///
/// So the widths stay and the claim is the other one, which is the claim
/// worth having: a cell's texture is the size of a cell in whichever
/// arrangement the bay is in, and a resize that changes that frees the
/// registration it replaces. Three sizes, and each is a different way of
/// getting it wrong:
///
/// - 112 x 63 in the row, which is the cell and not the row, the
///   region, the picture or the window.
/// - 252 x 142 beside the picture, which is the same rule read off a
///   column instead of a track — 35,784 texels against 7,056, which is
///   5.1x, remade once at the crossover and not once per frame of
///   the drag that crossed it.
/// - and the scale, which is the display it is dragged onto rather
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
        None,
        mcp::Slots::unpointed(),
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

/// The whole loop, closed on a real deck: a hand moves a knob and the strip
/// follows because the *deck* changed.
///
/// `tests/fader.rs` asserts everything up to the operation and one thing past
/// it — that the console keeps no value of its own — and it does all of that
/// with no deck anywhere, which is the point of that file. This is the other
/// end, and it needs a device because a `Deck` does: the operation becomes a
/// `Record`, the record moves the deck, and the strips are read back off the
/// deck by [`mixer`] exactly as the frame reads them.
///
/// The middle step is the one worth the device. Between the drag and the record
/// the strip must *not* have moved — if it had, the console would be showing a
/// number it kept rather than one the deck holds, and every assertion after it
/// would pass over a second copy of the deck's state.
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
        None,
        mcp::Slots::unpointed(),
    );

    // The strips, written the way the frame writes them.
    let material = vec![shipped().material(); engine.deck.slot_count()];
    let mut strips = Vec::new();
    mixer(&engine.deck, &material, &mut strips);
    assert_eq!(strips.len(), engine.deck.slot_count());
    let was = engine.deck.gain(karakuri_engine::DeckSlot(0));

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
    assert_eq!(engine.deck.gain(karakuri_engine::DeckSlot(0)), was);
    let mut after = Vec::new();
    mixer(&engine.deck, &material, &mut after);
    assert_eq!(
        after, strips,
        "the strip moved before the deck did, so the console is keeping a value"
    );

    // The record, and the deck.
    let record = super::tests::only_record(&operation);
    assert!(apply(
        &record,
        &mut engine.deck,
        &mut engine.look,
        &mut engine.chain
    )
    .is_some());
    assert_eq!(
        engine.deck.gain(karakuri_engine::DeckSlot(0)),
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
    engine.deck.set_gain(karakuri_engine::DeckSlot(0), 0.5);
    mixer(&engine.deck, &material, &mut after);
    assert_eq!(after[0].gain, 0.5);
}
/// A mix key moves the deck the operator selected, and leaves the other three
/// exactly where they were — the three key routes closed on a real deck, which
/// is why this is here rather than beside the helpers' own tests.
///
/// `tests::stepping_the_trim_…` and the one beside it assert [`gain_key`] and
/// [`opacity_key`] with no deck anywhere, which is the point of those two, and
/// `karakuri-console/tests/grammar.rs` asserts which control a press lands on
/// with no deck either. This is the other end of the same chain, and it needs a
/// device because a `Deck` does: four slots exist, the selection is one of
/// them, the level the press steps from is read off that slot, the operation
/// becomes a `Record` and the record moves one slot.
///
/// What separates it from a plausible wrong answer is which deck it lands on.
/// Deck C is selected — not the default and not the last, so a selection
/// ignored in either direction lands somewhere this test can see — and no two
/// slots are seeded alike, which is `tests/blend.rs`'s rule at the far end of
/// the same chain. Both halves are asserted: the one slot that moved and the
/// three that did not, because only the second catches a route that acted on a
/// deck of its own.
///
/// And the level comes off the deck rather than off `view::Strip`, which the
/// arms' own comment claims and nothing measured. A strip is that same reading
/// copied once a frame, so the two disagree the moment anything moves the deck
/// without the frame having run again — a scheduled fade landing between the
/// two is one way and the only one the comment names, but it is the *staleness*
/// that matters and not how it arose, so it is made here the cheap way. What is
/// asserted is that the two sources give different answers and that the deck's
/// is the one that lands right.
///
/// # What this cannot reach, and what does
///
/// This doc named three arms inline in `App::window_event` until 2026-09-10,
/// when ADR-0333 moved the trim and the fader's half of that chain into two
/// free functions, [`held`] and [`answered`], reached from the grammar rather
/// than from three letters. `answered` is not inline in `window_event` any
/// more, but it is still out of this test's reach for a narrower reason: it
/// takes `&mut Gfx`, which bundles a live `winit::window::Window` and a
/// `wgpu::Surface`, and nothing in this workspace builds one off-screen for a
/// test the way [`Engine`] is built here for a bare `Deck`. So this presses
/// [`held`], [`gain_key`] and [`opacity_key`] by hand, in the order
/// `answered`'s `Trim`/`Fader` arm calls them, rather than calling `answered`
/// itself.
/// `holding_reads_the_addressed_decks_own_state_and_never_a_strip_that_predates_it`,
/// below, presses [`holding`] — `answered`'s neighbour and the other function
/// ADR-0333 named — directly, because [`holding`] takes only `&Deck` and needs
/// no window at all. Between the two, every function the grammar's mix answers
/// call on a deck is pressed by something; only `answered`'s own dispatch —
/// that it calls them in this order, on the deck the address named — is still
/// asserted by hand here rather than by entering the function that actually
/// does it.
#[test]
fn a_mix_key_moves_the_deck_the_operator_selected_and_leaves_the_others_alone() {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const W: u32 = 1440;
    const H: u32 = 900;
    /// Deck C.
    const SELECTED: u8 = 2;
    /// Two levels are the same level, allowing for the arithmetic: a tenth is not
    /// an `f32`, so `0.6 + GAIN_STEP` and `0.7` are two different numbers and
    /// neither of them is wrong.
    const CLOSE: f32 = 1e-6;

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
        None,
        mcp::Slots::unpointed(),
    );
    let material = vec![shipped().material(); engine.deck.slot_count()];
    let here = usize::from(SELECTED);
    let here_slot = EngineSlot(SELECTED);
    assert!(
        here < engine.deck.slot_count(),
        "this deck has {} slots and the test selects deck {SELECTED}, so there is nothing \
         here to move",
        engine.deck.slot_count()
    );

    // **No two slots alike, in all three values.** An answer read off the
    // wrong slot is then a wrong answer rather than the right one by luck,
    // and the blends are seeded so the selected deck's next mode is not
    // where it already was.
    let seeded = [
        (0.20_f32, 0.90_f32, Blend::Add),
        (0.40, 0.70, Blend::Max),
        (0.60, 0.50, Blend::Over),
        (0.80, 0.30, Blend::Add),
    ];
    for (slot, (gain, opacity, blend)) in seeded.iter().copied().enumerate() {
        let addr = EngineSlot(slot as u8);
        engine.deck.set_gain(addr, gain);
        engine.deck.set_opacity(addr, opacity);
        engine.deck.set_blend(addr, blend);
    }

    // The console, with the strips the frame would have written into it —
    // `View::select` refuses a deck the mixer draws no strip for, so the
    // selection cannot be made before the readings are there.
    let mut readout = Readout::new(W as f32, H as f32);
    mixer(&engine.deck, &material, &mut readout.view.mixer);
    assert_eq!(
        readout.view.mixer.len(),
        engine.deck.slot_count(),
        "the view is not holding one strip per slot, so the selection below is being made \
         against something other than this deck"
    );
    assert_eq!(
        readout.view.selection(),
        0,
        "a run no longer opens on deck A, so selecting deck C proves nothing about a route \
         that ignores the selection"
    );
    assert!(
        readout.view.select(SELECTED),
        "deck C could not be selected on a four-slot deck"
    );
    assert_eq!(readout.view.selection(), SELECTED);

    /// Every value the three keys can move, per slot — the whole of what a press is
    /// allowed to have touched.
    fn snapshot(deck: &Deck) -> Vec<(f32, f32, Blend)> {
        (0..deck.slot_count())
            .map(|slot| {
                let addr = EngineSlot(slot as u8);
                (deck.gain(addr), deck.opacity(addr), deck.blend(addr))
            })
            .collect()
    }

    // -- the trim -------------------------------------------------------

    let was = snapshot(&engine.deck);
    let deck = readout.view.selection();
    let slot = held(&engine.deck, deck).expect("the selection is a slot this deck has");
    let gain = gain_key(Step::Up, engine.deck.gain(slot));
    let record = super::tests::only_record(&Operation::SetGain { deck, gain });
    assert!(apply(
        &record,
        &mut engine.deck,
        &mut engine.look,
        &mut engine.chain
    )
    .is_some());
    let now = snapshot(&engine.deck);
    assert!(
        (now[here].0 - (was[here].0 + GAIN_STEP)).abs() <= CLOSE,
        "an up press on deck C's addressed trim took it from {} to {} rather than one \
         {GAIN_STEP} up",
        was[here].0,
        now[here].0
    );
    for other in (0..now.len()).filter(|slot| *slot != here) {
        assert_eq!(
            now[other], was[other],
            "a press of `]` with deck C selected moved deck {other} as well — the route is \
             not addressed to the deck the operator picked"
        );
    }

    // -- the fader ------------------------------------------------------

    let was = snapshot(&engine.deck);
    let opacity = opacity_key(Step::Up, engine.deck.opacity(slot));
    let record = super::tests::only_record(&Operation::SetOpacity { deck, opacity });
    assert!(apply(
        &record,
        &mut engine.deck,
        &mut engine.look,
        &mut engine.chain
    )
    .is_some());
    let now = snapshot(&engine.deck);
    assert!(
        (now[here].1 - (was[here].1 + OPACITY_STEP)).abs() <= CLOSE,
        "an up press on deck C's addressed fader took it from {} to {} rather than one \
         {OPACITY_STEP} up",
        was[here].1,
        now[here].1
    );
    for other in (0..now.len()).filter(|slot| *slot != here) {
        assert_eq!(
            now[other], was[other],
            "a press on deck C's fader moved deck {other} as well"
        );
    }

    // -- the blend ------------------------------------------------------

    let was = snapshot(&engine.deck);
    let blend = karakuri_console::view::after(blend_mode(engine.deck.blend(slot)));
    assert_ne!(
        blend,
        blend_mode(was[here].2),
        "deck C's next mode is the one it is already on, so the assertion below would pass \
         on a press that did nothing"
    );
    let record = super::tests::only_record(&Operation::SetBlendMode { deck, blend });
    assert!(apply(
        &record,
        &mut engine.deck,
        &mut engine.look,
        &mut engine.chain
    )
    .is_some());
    let now = snapshot(&engine.deck);
    assert_eq!(
        blend_mode(now[here].2),
        blend,
        "`space` on deck C's addressed blend chip did not arrive at the mode the cycle \
         names"
    );
    for other in (0..now.len()).filter(|slot| *slot != here) {
        assert_eq!(
            now[other], was[other],
            "a press on deck C's blend chip moved deck {other} as well"
        );
    }

    // -- the guard ------------------------------------------------------

    // **[`held`] is what stands between an event handler and an abort**,
    // and this is the device half of that sentence: `Deck::gain` indexes
    // its slots, so the thing the guard refuses is a real panic and not a
    // supposed one. A panic here would take the process with it rather
    // than unwinding into a message — see the module documentation — which
    // is why the arms ask before they read.
    let past = engine.deck.slot_count();
    for slot in 0..past {
        assert_eq!(
            held(&engine.deck, slot as u8),
            Some(EngineSlot(slot as u8)),
            "slot {slot} is one this deck has and the guard refused it, so every press \
             would return without doing anything"
        );
    }
    assert_eq!(
        held(&engine.deck, past as u8),
        None,
        "the guard let a selection past this deck's {past} slots through"
    );
    assert_eq!(held(&engine.deck, u8::MAX), None);
    let quiet = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let read = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine.deck.gain(EngineSlot(past as u8))
    }));
    std::panic::set_hook(quiet);
    assert!(
        read.is_err(),
        "reading past this deck's slots answered {read:?} instead of panicking, so the \
         guard above is standing in front of nothing and this test says nothing about why \
         it is there"
    );

    // -- the reading is the deck's, not the strip's ----------------------

    // The strips as a frame would have left them, and then the deck moved
    // with no frame in between — which is what a scheduled fade landing
    // between the two does, arrived at the cheap way because it is the
    // staleness that matters rather than how it arose.
    let mut strips = Vec::new();
    mixer(&engine.deck, &material, &mut strips);
    assert_eq!(
        strips[here].gain,
        engine.deck.gain(here_slot),
        "the strips were just read off the deck and already disagree with it"
    );
    engine.deck.set_gain(here_slot, 0.25);
    assert_ne!(
        strips[here].gain,
        engine.deck.gain(here_slot),
        "the deck moved and the copy the console is holding moved with it, so there is no \
         stale reading here to tell the two sources apart"
    );
    let off_the_deck = gain_key(Step::Up, engine.deck.gain(here_slot));
    let off_the_strip = gain_key(Step::Up, strips[here].gain);
    assert_ne!(
        off_the_deck, off_the_strip,
        "a step counted from the deck and a step counted from the strip came out at the \
         same place, so this test cannot tell which source a press used"
    );
    let record = super::tests::only_record(&Operation::SetGain {
        deck,
        gain: off_the_deck,
    });
    assert!(apply(
        &record,
        &mut engine.deck,
        &mut engine.look,
        &mut engine.chain
    )
    .is_some());
    assert!(
        (engine.deck.gain(here_slot) - (0.25 + GAIN_STEP)).abs() <= CLOSE,
        "a press stepping from the deck's own reading landed at {} rather than at {}",
        engine.deck.gain(here_slot),
        0.25 + GAIN_STEP
    );
    assert!(
        (engine.deck.gain(here_slot) - off_the_strip).abs() > CLOSE,
        "the press landed where a step off the stale strip would have put it"
    );
}

/// `holding` reads the addressed deck's own state, and never a strip that
/// predates it — the two functions'
/// `the_grammars_mix_answers_act_on_the_addressed_deck_and_read_it_off_the_deck`
/// used to hold as a claim about this file's text, read as a claim about what
/// runs.
///
/// [`holding`] hands the console the three states a mixer strip cycles —
/// [`tally`], [`blend_mode`] and [`masked`] applied to
/// `Deck::requested_residency`, `Deck::blend` and `Deck::mask` — plus the angle
/// carried through unchanged. A text scan can only say those calls are *spelled
/// somewhere above the tests*; this presses [`holding`] itself and checks the
/// *values* it hands back, against a slot moved after a strip had already
/// copied its old ones — the same staleness
/// `a_mix_key_moves_the_deck_operator_selected_and_leaves_the_others_alone`
/// presses [`gain_key`] against, above.
///
/// Nothing here reaches for a strip because nothing here has one to reach for.
/// [`holding`]'s only parameters are `&Deck` and a slot number, and
/// `karakuri-engine` does not depend on `karakuri-console` (ADR-0156): there is
/// no `view::Strip` in scope for a function with this signature to name, by
/// accident or otherwise. That half of the old claim is a fact about the crate
/// graph, settled the day this file stopped being allowed to import the
/// engine's own compositor into the console — not something either the old scan
/// or this test has to hold at runtime. What is worth pressing is the other
/// half: that the values [`holding`] reports are the ones on the deck now.
#[test]
fn holding_reads_the_addressed_decks_own_state_and_never_a_strip_that_predates_it() {
    let gpu = Gpu::headless().expect("no GPU");
    let mut renderer = egui_wgpu::Renderer::new(
        &gpu.device,
        wgpu::TextureFormat::Rgba8Unorm,
        egui_wgpu::RendererOptions::default(),
    );
    let mut panel = Panel::new(1440.0, 900.0);
    panel.solve();
    let mut engine = Engine::new(
        &gpu,
        &mut renderer,
        &shipped_slots(),
        panel.layout(),
        1.0,
        None,
        None,
        mcp::Slots::unpointed(),
    );
    let material = vec![shipped().material(); engine.deck.slot_count()];
    /// Deck C — not the default and not the last, [`held`]'s own reason for the mix
    /// key test above.
    const AT: u8 = 2;
    let slot = held(&engine.deck, AT).expect("this deck has a slot 2");

    // A frame's worth of strips, read before anything below moves the
    // deck — the copy `holding` must answer differently from once the
    // deck has moved on.
    let mut strips = Vec::new();
    mixer(&engine.deck, &material, &mut strips);
    let before = strips[usize::from(AT)].clone();

    // Every state `holding` reports, moved to something the frame above
    // never saw.
    engine.deck.set_residency(slot, Residency::Priming);
    engine.deck.set_blend(slot, Blend::Max);
    engine
        .deck
        .set_mask(slot, Mask::new(MaskKind::Radial, 0.75, 0.0, 0.0));
    assert_ne!(
        tally(engine.deck.requested_residency(slot)),
        before.requested,
        "deck C's next residency is the one the frame above already drew, so the assertion \
         below would pass on a read that never moved"
    );
    assert_ne!(
        blend_mode(engine.deck.blend(slot)),
        before.blend,
        "deck C's next blend is the one the frame above already drew"
    );
    assert_ne!(
        masked(engine.deck.mask(slot).kind()),
        before.mask,
        "deck C's next mask is the one the frame above already drew"
    );
    assert_ne!(
        engine.deck.mask(slot).angle(),
        before.mask_angle,
        "deck C's next mask angle is the one the frame above already drew"
    );

    let now = holding(&engine.deck, AT).expect("this deck has a slot 2");
    assert_eq!(
        now.requested,
        tally(engine.deck.requested_residency(slot)),
        "`holding` answered a residency other than the one the deck holds now"
    );
    assert_eq!(
        now.blend,
        blend_mode(engine.deck.blend(slot)),
        "`holding` answered a blend other than the one the deck holds now"
    );
    assert_eq!(
        now.mask,
        masked(engine.deck.mask(slot).kind()),
        "`holding` answered a mask shape other than the one the deck holds now"
    );
    assert_eq!(
        now.mask_angle,
        engine.deck.mask(slot).angle(),
        "`holding` answered a mask angle other than the one the deck holds now"
    );

    // And none of the three states agrees with the strip a frame drew
    // before the move — the stale reading `holding` must not be
    // answering from.
    assert_ne!(now.requested, before.requested);
    assert_ne!(now.blend, before.blend);
    assert_ne!(now.mask, before.mask);
    assert_ne!(now.mask_angle, before.mask_angle);
}

/// Every slot is its own simulation of the one procedure, which is what keeps a
/// mixer of four channels from being one picture drawn four times.
///
/// [`slot_salt`] is the derivation and this asserts it where it lands: the salt
/// is read back off the `Set` the deck actually built, which
/// `Set::source_salts` exists for — *"a caller that assigned none finds out
/// what it got"* — so a slot built at the wrong seed, or four slots built at
/// one, fails here rather than in a picture only an operator with two channels
/// up would ever notice. Read off the deck rather than by calling `slot_salt`
/// again, which would be the test agreeing with itself about the one thing it
/// checks.
///
/// It is deck A's salt that is named against a constant, because that one is a
/// claim about a *value* — 7 is what `karakuri-cli`'s own tests use, so this
/// program's picture looks like theirs. The rest is a claim about distinctness,
/// and distinctness is what is asserted.
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
        None,
        mcp::Slots::unpointed(),
    );

    assert_eq!(
        engine.deck.slot_count(),
        SLOTS,
        "the deck is not built full, so the mixer has tracks with no channel in them"
    );
    let salts: Vec<u32> = (0..engine.deck.slot_count())
        .map(|slot| {
            let salts = engine
                .deck
                .slot(EngineSlot(slot as u8))
                .set()
                .source_salts();
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

/// A parked deck is reachable by running this window, and both of its
/// residencies reach the strips.
///
///
/// [ADR-0190](../../../docs/adr/0190-the-parked-tally-rolls-because-two-lamps-do-not-fit-in-fifty-three-pixels.md)
/// drew a chip that rolls while a request is outstanding and `tests/parked.rs`
/// asserts every pixel of it — from strips written by hand. This is the other
/// question, and it was the one answered `no`: whether the engine can put this
/// panel in that state at all. A `Deck` grants every residency it is asked for
/// until something governs, so before [`Engine::ask_to_prime`] the two halves
/// of the pair could not disagree here however long anybody ran the program,
/// and the animation that is fully tested was unreachable in the one place a
/// person would look at it.
///
/// Every step is the product's: two Sets are built, `Deck::measure_slots`
/// measures them with a real probe, the budget is set from what it measured,
/// [`Deck::govern`] refuses, and [`mixer`] reads the two residencies back off
/// the deck the way the frame does. The reason is asserted and not only the
/// park, because three of the four reasons that satisfy `Deck::is_parked` mean
/// this program forgot to do something — `Unmeasured` and `CommittedUnknown`
/// are a probe that never ran, and `NoPrimingNeeded` is a closed-form Set that
/// never needed warming. Only `NoHeadroom` is the budget refusing.
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
    // **The reference workload's pair rather than the shipped one**, and
    // that is the whole of what keeps this test about the budget: see
    // [`reference`]. `examples/star_vortex.kset`'s pair is closed-form, so
    // this deck built from it parks deck B for `NoPrimingNeeded` and the
    // assertion below would be reading a different refusal.
    let reference = reference();
    let slots: Vec<Sources> = std::iter::repeat_n(reference.clone(), SLOTS).collect();
    let mut engine = Engine::new(
        &gpu,
        &mut renderer,
        &slots,
        panel.layout(),
        1.0,
        None,
        None,
        mcp::Slots::unpointed(),
    );
    let material = vec![reference.material(); engine.deck.slot_count()];

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
        engine.deck.is_parked(EngineSlot(ASKED_TO_PRIME as u8)),
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
        engine.deck.residency(EngineSlot(ON_AIR as u8)),
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
    // **Against what deck A is *budgeted* on rather than what it was
    // measured at**, which since
    // [ADR-0296](../../../docs/adr/0296-the-governor-budgets-on-the-estimate-where-it-answers-and-on-the-measurement-where-it-does-not.md)
    // are two different numbers: the governor spends the estimate where a
    // Set has one and the measurement where it does not, and this test is
    // about *which slots* are in the sum rather than about which reading
    // each of them contributed. Taking it off the decision is also what
    // stops this assertion passing by arithmetic coincidence the day the
    // estimate stops arriving — the number it compares against moves with
    // the same rule the sum is built from.
    let budgeted = governed
        .decisions
        .iter()
        .find(|decision| decision.slot == ON_AIR)
        .and_then(|decision| decision.budgeted_ms)
        .expect("deck A is governed and has a number to be budgeted on");
    assert!(
        (governed.committed_ms - budgeted).abs() < f32::EPSILON,
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
    // P-0095: it carries how it was taken — `Deck::measure_slots`, one
    // `Probe` for the deck, at the probe's own resolution rather than at
    // `CANVAS`.
    let costs: Vec<f32> = (0..SLOTS)
        .filter_map(|slot| engine.deck.slot(EngineSlot(slot as u8)).measured_cost())
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

/// The whole loop, closed on the transition row: the settings the three pills
/// arrive at are what a press on `go` converts against, and the wipe writes its
/// records rather than answering `Owed`.
///
/// `karakuri-console/tests/transition.rs` asserts everything up to the
/// operation with no deck anywhere, which is the point of that file. This is
/// the other end, and it needs a device for the reason the mask mini's test
/// does: a `Deck` is what the mask, the blend, the residency and the grid are
/// read off, and a wipe reads three of those four.
///
/// The path is the product's own from end to end. The shape, the grid and the
/// length are walked to where this test wants them through [`scheduled`] — the
/// same function the press handler calls — rather than written into the view,
/// so what is converted is a setting the console took. The press is
/// [`TransitionRow::go`] at the capsule's centre. The reading is [`reading`]'s,
/// unaltered.
///
/// What separates this from a plausible wrong answer is the `Owed` arm. Until
/// this window supplied `Current::transition` and `Current::mix` a wipe came
/// back `Owed(NotRead(…))` — a sentence rather than a fade — and every
/// assertion below about the records would have been unreachable. So the answer
/// is asserted to be `Records` before anything is read out of it, and the
/// transition record is checked against the settings *the row is on* rather
/// than against numbers written here: a conversion that invented a quantum
/// would otherwise agree with a test that invented the same one.
///
/// And the mask position is asserted to still be owed, off the same deck, which
/// is the half
/// `tests::an_operation_whose_record_is_owed_is_said_rather_than_swallowed`
/// cannot make with no device: that test spells `Current::default()` for it,
/// and this is what says the empty reading and the one this window takes are
/// the same value.
#[test]
fn the_go_pill_runs_a_wipe_against_the_settings_the_row_is_on() {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const W: u32 = 1440;
    const H: u32 = 900;
    /// The deck the wipe covers, and the one the selection is put on.
    const UNDER: usize = 1;

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
        None,
        mcp::Slots::unpointed(),
    );
    let material = vec![shipped().material(); engine.deck.slot_count()];
    let over = (UNDER + 1) % engine.deck.slot_count();
    let over_slot = EngineSlot(over as u8);
    assert!(
        engine.deck.slot_count() >= 2,
        "a deck of one slot cannot wipe and this test needs one that can"
    );

    // **The grid is run on before anything is scheduled**, and it is the
    // one piece of setup here that is not the product's own path.
    // `quantise` answers `ceil(beats / quantum) * quantum`, so at beat
    // zero every quantum agrees on zero and a record built with the wrong
    // one is indistinguishable from a record built with the row's. Four
    // seconds of grid is past the first bar at the default tempo, and the
    // guard below is what says this line is still doing its job.
    let mut signals = karakuri_engine::binding::Signals::default();
    signals.advance(u8::MAX, 1.0 / 60.0);
    engine.deck.set_signals(signals);
    assert!(
        engine.deck.signals().oscillator().beats() > 1.0,
        "the grid is still on the first beat, so every quantum quantises to the same \
         instant and the start below says nothing about the row"
    );

    let mut view = View::new(Room::Day);
    mixer(&engine.deck, &material, &mut view.mixer);
    view.select(UNDER as u8);
    assert_eq!(view.selection(), UNDER as u8);

    // **The row walked to `iris · now · 2 beats`, through the door the
    // panel has.** `now` rather than the next bar so the record's start is
    // the beat the grid is on and the arithmetic below has nothing to wait
    // for; every value is still one the pills can reach.
    for setting in [
        karakuri_operation::TransitionSetting::WipeShape {
            kind: karakuri_operation::WipeKind::Radial,
            angle: 0.0,
        },
        karakuri_operation::TransitionSetting::Quantum { beats: 0.0 },
        karakuri_operation::TransitionSetting::Length { beats: 2.0 },
    ] {
        let said = scheduled(&mut view, &Operation::SetTransition { setting })
            .expect("a transition setting said nothing at all");
        assert!(
            !said.contains("refused"),
            "the console refused `{setting:?}`, which this test needs it to take: {said}"
        );
    }
    let settings = view.transition();
    assert!(
        settings.armed(),
        "the row is not armed, so the press below would be refused for the shape"
    );

    // The press, at the centre of the `go` capsule.
    let ctx = super::tests::drawn_once();
    let row = transition_row(&ctx, panel.layout(), settings).expect("the transition row");
    let centre = row.go.center();
    let asked = row
        .go(
            Point::new(centre.x, centre.y),
            view.selection(),
            view.mixer.len(),
        )
        .expect("a press on the `go` capsule");
    let Go::Wipe(operation) = asked else {
        panic!(
            "the press was refused with the row armed and {} strips: {asked:?}",
            view.mixer.len()
        )
    };
    assert_eq!(
        operation,
        Operation::Wipe {
            from: UNDER as u8,
            to: over as u8,
        },
        "the press did not cover the addressed deck with the next one round"
    );

    // **Nothing has been told anything yet**, which is the middle step the
    // device is worth: the console asks and the deck moves when the record
    // does.
    let before = engine.deck.mask(over_slot);
    assert_eq!(
        engine.deck.transitions_on(over_slot).count(),
        0,
        "something was already moving on the deck this wipe arrives on"
    );

    // The reading, and the answer that used to be a sentence.
    let current = reading(
        &operation,
        &engine.deck,
        &engine.look,
        &engine.chain,
        settings,
    );
    let wiped = written(&operation, &current);
    assert!(
        matches!(wiped, Written::Records(_)),
        "a wipe off this window's own reading did not write records: {wiped:?} — the \
         two readings a wipe converts against are what this asserts are handed over"
    );
    let Written::Records(records) = &wiped else {
        unreachable!("just matched")
    };

    // **The transition the row is on, and not one this test spelled.**
    // `now` is a quantum of 0, which `quantise` answers with the beat the
    // grid is on, so the start is read off the same oscillator the
    // conversion read.
    let start = karakuri_engine::transition::quantise(
        engine.deck.signals().oscillator().beats(),
        settings.quantum,
    );
    assert!(
        records.contains(&Record::Transition {
            slot: DeckSlot(over as u8),
            control: "mask".to_owned(),
            to: 1.0,
            start,
            beats: settings.length,
            curve: "smooth".to_owned(),
        }),
        "the wipe's scheduled move is not the one the row is set to — {records:?}"
    );
    // **The guard on the start**, and it is what a green run means here: a
    // quantum this test did not choose would put the move on a different
    // instant, and without a grid that has been running it would put it on
    // the same one. `next bar` is the pill's other end of the same cycle.
    assert_ne!(
        start,
        karakuri_engine::transition::quantise(engine.deck.signals().oscillator().beats(), 4.0),
        "`now` and `next bar` quantise to the same instant on this grid, so the start \
         above is satisfied by a conversion reading a quantum nobody chose"
    );
    // And the mask, written whole out of the shape the *row* holds and the
    // position and soft edge the *deck* is wearing. The second of the two
    // puts the front at 0, which is what makes the move a wipe.
    assert!(
        records.contains(&Record::Mask {
            slot: DeckSlot(over as u8),
            kind: "radial".to_owned(),
            angle: 0.0,
            position: 0.0,
            softness: before.softness(),
        }),
        "the front is not put to 0 at the shape the row holds, with the deck's own soft \
         edge — {records:?}"
    );

    // And the deck follows.
    for record in records {
        apply(
            record,
            &mut engine.deck,
            &mut engine.look,
            &mut engine.chain,
        );
    }
    assert_eq!(
        engine.deck.mask(over_slot).kind(),
        MaskKind::Radial,
        "the records were built and the arriving deck is not wearing the row's shape"
    );
    assert_eq!(
        engine.deck.transitions_on(over_slot).count(),
        1,
        "the wipe wrote its records and nothing is moving on the deck it arrives on"
    );

    // **And the gap that was here until 2026-09-10, closed off the same
    // deck.** [`reading`] answered for a shape and for a wipe's arriving
    // deck and not for a position, so `written` came back
    // `Owed(NotRead(Mask))` and nothing moved — ADR-0334 recorded it and
    // ADR-0341 fixed it with one arm. This is the other side of that
    // assertion: the position is written whole, out of the number the
    // operation carries and the shape, the angle and the soft edge the
    // *deck* is wearing.
    //
    // **Read off this deck rather than spelled**, which is what makes it
    // the half `an_operation_whose_record_is_owed_is_said_rather_than_swallowed`
    // cannot make: a conversion that took a default here would pass
    // against a hand-written `Current` and put a shape nobody chose on a
    // deck mid-wipe.
    let wearing = engine.deck.mask(over_slot);
    let (kind, angle, softness) = (wearing.kind(), wearing.angle(), wearing.softness());
    let front = Operation::SetMaskPosition {
        deck: over as u8,
        position: 0.5,
    };
    assert_eq!(
        written(
            &front,
            &reading(&front, &engine.deck, &engine.look, &engine.chain, settings)
        ),
        Written::Records(vec![Record::Mask {
            slot: DeckSlot(over as u8),
            kind: wipe_kind(kind).name().to_string(),
            angle,
            position: 0.5,
            softness,
        }]),
        "a mask position off a real deck did not come back as the deck's own mask with \
         the asked-for front in it — the reading ADR-0341 added is not being taken, or \
         it is being taken off the wrong slot"
    );
}

/// The whole loop, closed on a parked deck: a press on the tally chip withdraws
/// the prime request the governor could not grant, and the strip stops rolling
/// because the *deck* changed.
///
/// `tests/tally.rs` asserts everything up to the operation with no deck
/// anywhere, which is the point of that file. This is the other end, and it
/// needs a device because a `Deck` does — and because the state under test is
/// one only a governor pass can produce
/// ([ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)):
/// the request is asked for through the product's own path, refused by a budget
/// computed from what the probe measured, and read back off the deck by
/// [`mixer`] exactly as the frame reads it.
///
/// What separates this from a plausible wrong answer is which residency the
/// press names. The parked strip *shows* `alloc` and was *asked for* `prim`. A
/// chip cycling from what it shows would ask for `live` and put deck B on air;
/// cycling from the request asks for `allocated`, which is the withdrawal — and
/// both are asserted here, on the operation and again on the deck, because the
/// two are the same mistake at two removes (ADR-0195).
///
/// The middle step is the one worth the device, as in the fader's test: between
/// the press and the record the deck must not have moved, or the console would
/// be applying what it is only supposed to ask for.
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
        None,
        mcp::Slots::unpointed(),
    );
    let material = vec![shipped().material(); engine.deck.slot_count()];

    // The state, produced by the governor and not written here.
    let governed = engine.ask_to_prime(&gpu);
    assert!(
        engine.deck.is_parked(EngineSlot(ASKED_TO_PRIME as u8)),
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
    assert!(engine.deck.is_parked(EngineSlot(ASKED_TO_PRIME as u8)));
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
            slot: DeckSlot(ASKED_TO_PRIME as u8),
            level: "allocated".to_owned(),
        }
    );
    assert!(apply(
        &record,
        &mut engine.deck,
        &mut engine.look,
        &mut engine.chain
    )
    .is_some());
    assert_eq!(
        engine
            .deck
            .requested_residency(EngineSlot(ASKED_TO_PRIME as u8)),
        Residency::Allocated,
        "the record was built and the request did not move, so the control ends nowhere"
    );
    assert_eq!(
        engine.deck.residency(EngineSlot(ASKED_TO_PRIME as u8)),
        Residency::Allocated,
        "the withdrawal left the slot somewhere other than where it was being held"
    );
    assert!(
        !engine.deck.is_parked(EngineSlot(ASKED_TO_PRIME as u8)),
        "the slot is still parked, so the request was not withdrawn"
    );
    assert_eq!(
        engine.deck.residency(EngineSlot(ON_AIR as u8)),
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
        slot: DeckSlot(ASKED_TO_PRIME as u8),
        level: "priming".to_owned(),
    };
    assert!(apply(
        &again,
        &mut engine.deck,
        &mut engine.look,
        &mut engine.chain
    )
    .is_some());
    assert_eq!(
        engine
            .deck
            .requested_residency(EngineSlot(ASKED_TO_PRIME as u8)),
        Residency::Priming,
        "the request was not written"
    );
    assert!(
        engine.deck.is_parked(EngineSlot(ASKED_TO_PRIME as u8)),
        "the prime request was granted rather than parked, so nothing governed the record \
         and the panel is drawing a residency the budget never allowed"
    );
}

/// The whole loop, closed on a masked deck: a press on the mask mini chooses
/// the next shape and leaves the front, the soft edge and — the one this test
/// exists for — the *angle* exactly where they were.
///
/// `tests/mask.rs` asserts everything up to the operation with no deck
/// anywhere, which is the point of that file. This is the other end, and it
/// needs a device because a `Deck` does.
///
/// What separates this from the plausible wrong answer is the angle. The chip
/// names a *shape*; `Operation::SetMaskShape` carries a shape and an angle
/// (ADR-0201), so a press must carry an angle it does not control. Carrying the
/// one the slot already wears is the whole of
/// [ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md);
/// carrying `0.0` would look like a chip minding its own business and would
/// straighten a diagonal wipe on every press, with nothing on the panel saying
/// so — the mark is the same mark at any angle.
///
/// The middle step is the one worth the device, as in the fader's test and the
/// tally's: between the press and the record the deck must not have moved, or
/// the console would be applying what it is only supposed to ask for.
#[test]
fn a_press_on_the_mask_mini_chooses_a_shape_and_keeps_the_angle() {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const W: u32 = 1440;
    const H: u32 = 900;
    /// A diagonal front, in radians — not zero, and not the default, which is the
    /// only reason this test can tell the two designs apart.
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
        None,
        mcp::Slots::unpointed(),
    );
    let material = vec![shipped().material(); engine.deck.slot_count()];

    // A wipe in progress on the deck that is on air: a straight front,
    // running at an angle, part of the way across.
    engine.deck.set_mask(
        EngineSlot(ON_AIR as u8),
        Mask::new(MaskKind::Linear, ANGLE, FRONT, SOFTNESS),
    );
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
    assert_eq!(
        engine.deck.mask(EngineSlot(ON_AIR as u8)).kind(),
        MaskKind::Linear
    );
    let mut after = Vec::new();
    mixer(&engine.deck, &material, &mut after);
    assert_eq!(
        after, strips,
        "the strip moved before the deck did, so the console is keeping a value"
    );

    // The record, out of the operation and the reading the harness takes
    // off the deck — written **whole**, which is the half the operation
    // does not name (ADR-0201).
    // **The transition settings are where a run begins and this press
    // does not read them**: `Current::transition` is a wipe's, a fade's, a
    // crossfade's and a selection's, and none of the three conversions
    // below is one of those. Handed in because `reading` takes them, and
    // `START` rather than a chosen value so that nothing here can look
    // like a setting the test needed.
    let written = written(
        &operation,
        &reading(
            &operation,
            &engine.deck,
            &engine.look,
            &engine.chain,
            TransitionSettings::START,
        ),
    );
    let Written::Records(records) = &written else {
        panic!("a press on the mask mini wrote no record: {written:?}")
    };
    assert_eq!(
        records.as_slice(),
        [Record::Mask {
            slot: DeckSlot(ON_AIR as u8),
            kind: "radial".to_owned(),
            angle: ANGLE,
            position: FRONT,
            softness: SOFTNESS,
        }],
        "the record is not the whole mask with only the shape changed"
    );

    // And the deck.
    assert!(apply(
        &records[0],
        &mut engine.deck,
        &mut engine.look,
        &mut engine.chain
    )
    .is_some());
    let mask = engine.deck.mask(EngineSlot(ON_AIR as u8));
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

/// The whole loop, closed on the look: a press on the tone map capsule chooses
/// the next operator and keeps the level, and a press on the exposure track
/// sets the level and keeps the operator.
///
/// `tests/look.rs` asserts everything up to the operation with no engine
/// anywhere, which is the point of that file. This is the other end, and it
/// needs a device because [`Engine`] does — and because the value being moved
/// is [`Engine::look`], which is what every sink is drawn under.
///
/// What separates this from the plausible wrong answer is the third of the
/// record neither press names. `Record::Look` is an operator, a level and a
/// white point; each control asks for one of the first two and [`reading`]
/// supplies the rest (ADR-0192). A build that filled the missing thirds from a
/// default would cycle the tone map and silently reset the exposure — and would
/// rewrite `white_point`, which is on no surface at all and would therefore
/// change with nothing saying so. So the look this starts from has none of the
/// three at its default.
///
/// The middle step is the one worth the device, as in the mask's test: between
/// the press and the record the look must not have moved, or the console would
/// be applying what it is only supposed to ask for.
#[test]
fn a_press_on_the_look_controls_moves_the_look_every_sink_is_drawn_under() {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const W: u32 = 1440;
    const H: u32 = 900;
    /// Not `LOOK`'s three, so a press that dropped a third of the record and filled
    /// it from a default is visible in every one of them.
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
        None,
        mcp::Slots::unpointed(),
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
        health: Some(view::Stage::Landed),
        rec: Some(view::Rec::Idle),
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
        view.tracker,
        None,
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

    // **The transition settings are where a run begins and this press
    // does not read them**: `Current::transition` is a wipe's, a fade's, a
    // crossfade's and a selection's, and none of the three conversions
    // below is one of those. Handed in because `reading` takes them, and
    // `START` rather than a chosen value so that nothing here can look
    // like a setting the test needed.
    let chosen = written(
        &operation,
        &reading(
            &operation,
            &engine.deck,
            &engine.look,
            &engine.chain,
            TransitionSettings::START,
        ),
    );
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
    assert!(apply(
        &records[0],
        &mut engine.deck,
        &mut engine.look,
        &mut engine.chain
    )
    .is_some());
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
        view.tracker,
        None,
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

    // **The transition settings are where a run begins and this press
    // does not read them**: `Current::transition` is a wipe's, a fade's, a
    // crossfade's and a selection's, and none of the three conversions
    // below is one of those. Handed in because `reading` takes them, and
    // `START` rather than a chosen value so that nothing here can look
    // like a setting the test needed.
    let levelled = written(
        &operation,
        &reading(
            &operation,
            &engine.deck,
            &engine.look,
            &engine.chain,
            TransitionSettings::START,
        ),
    );
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
    assert!(apply(
        &records[0],
        &mut engine.deck,
        &mut engine.look,
        &mut engine.chain
    )
    .is_some());
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

/// Which rectangle each sink's texture is sized from, and where the console
/// then draws it — asked of the call the frame actually makes.
///
/// This is the hole `docs/roadmap.md` recorded, closed. The decision used to be
/// two `match`es inside `App::window_event`, and `winit` will not hand a test
/// an `ActiveEventLoop`, so nothing could call it: `mod gpu` asserted what
/// `Engine::new` did and not what the frame chose. Sizing deck A's texture from
/// the picture's rectangle was injected there and every test still passed. It
/// is [`aims`] and [`Engine::aim`] now, which take a solved layout and a scale
/// factor and touch no window, and this asks them at a viewport and a scale
/// neither of which `Engine::new` was given — so what is asserted is what `aim`
/// decided rather than what construction left behind.
///
/// Every half of it fails silently. A texture sized from the wrong rectangle
/// looks perfectly correct — the cell is drawn at whatever size it is and the
/// texture fills it — and is four to twenty times the texels the cell needs,
/// per frame, for as long as the deck runs. A `Picture` carrying an id from
/// before a resize is a freed registration, which `egui` draws as nothing at
/// all. And a folded region whose sink still acquires is the manual's *"no
/// state where it is hidden and still costing a pass"* quietly stopping being
/// true.
#[test]
fn the_frame_aims_each_sink_at_its_own_rectangle() {
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const W: u32 = 1440;
    const H: u32 = 900;
    /// A display of a different scale, because the size is the rectangle and the
    /// scale and a test at 1.0 cannot tell them apart.
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
        None,
        mcp::Slots::unpointed(),
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
    let (picture, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), SCALE, None);

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
    // every one of them is aimed at a cell, which is ADR-0258 —
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
    let (picture, previews) = engine.aim(&gpu, &mut renderer, panel.layout(), SCALE, None);
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

/// ADR-0155's bet, as an assertion.
///
/// The record chose `egui` and paid a `wgpu` major version for it on the
/// grounds that the panel and the engine share one `Device`. This builds the
/// console's frame with an `egui` context, tessellates it, renders it through
/// `egui-wgpu` into a texture on a device `karakuri-engine` created, and reads
/// the texels back. If the two ever resolve different `wgpu`s it does not
/// compile; if the render path breaks, `poll` gives a device-side complaint
/// somewhere to surface; and if what lands is not the console, the two pixels
/// below say so.
///
/// The pixels are the point. A frame that renders without complaining and is
/// the wrong colour is the failure that is easy to ship: `egui`'s shader writes
/// gamma-encoded texels because it is told the target is gamma space, so an
/// sRGB target encodes a second time and the whole panel washes out — with no
/// error anywhere. So a bay's body is asserted to be exactly `--c-panel` and a
/// divider is asserted not to be.
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
    let user = renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &primitives, &screen);
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
