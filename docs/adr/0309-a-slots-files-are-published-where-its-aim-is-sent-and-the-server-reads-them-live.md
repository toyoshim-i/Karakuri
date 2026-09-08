---
id: 0309
title: A slot's files are published where its aim is sent, and the server reads them live
status: accepted
date: 2026-09-08
supersedes: []
superseded_by: []
principles: [0083, 0085, 0087, 0091, 0094]
tags: [mcp, environment, panel, store, library]
---

# A slot's files are published where its aim is sent, and the server reads them live

## Context

`crates/karakuri` built the `mcp::Slots` it handed `mcp::serve` out of the **launch** working
copies, in `main`, before the window opened, and never wrote it again. A library load re-points a
deck: `loading` writes the Set's procedures into `<store>/scratch/` under new names — `B0-grid.kir`
rather than `B1-launch.kir` — and sends one `watch::Aim` naming them
([ADR-0228](0228-a-library-load-re-points-the-slots-source-and-never-installs-a-set.md),
[ADR-0304](0304-the-set-a-version-is-filed-under-rides-the-aim-that-re-points-the-slot.md)). The
server's copy did not move with it.

**So from the first load onwards, every address that surface resolved was the layout the deck had
stopped running.** `read_procedure` handed a model the source of a procedure the slot was no longer
playing; `write_procedure` wrote a file no watcher was polling and answered *compiled and written
to slot 1 L4:0 … it is being built*; and a node the loaded Set does hold was refused for not
existing, in a sentence naming the launch pair's nodes. **Not one of the three fails.** They are
plausible wrong answers on the one surface whose reader is a program in a loop, which is
[P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)'s
*a silently wrong picture loses to a loud failure* read one level away from the picture.

[ADR-0308](0308-the-library-bays-fifth-chip-walks-one-sets-history-and-a-row-lands-that-version-on-a-node.md)
met this and worked around it for one caller. Its landing needed the same `(slot, layer, index)`
walk, so `restored` built a **second** `Slots` out of `Aiming::at` at the moment of the press and
handed that to `put_back`. That was right about where the answer lives and wrong about how many
there should be: the workaround left two derivations of *which files is this slot running*, one
correct and one stale, and the stale one was the one a model was talking to.

**`Aiming::at` is already the derivation.** ADR-0304 settled that when it refused a per-slot
`Option<String>` beside the deck: an aim is every field of a slot's identity, a rewiring restates it
through `Aiming::re_aim` without passing through the load path at all, and a copy kept beside it
goes out of step at the second route. What was missing was not an answer but a way for it to cross
to a thread that cannot see the host's `Vec<Aiming>`.

**That mechanism exists and is next to this one in the same argument list.**
`karakuri_environment::Opening` is a shared handle over the operator's four classes: `main` makes
one, the bay-head pills write it, `serve` takes a clone, and the server reads it on **every call**,
because a class the operator closed between two calls has to be closed for the second. Its own head
says why a snapshot could not have implemented the decision at all. *Which files is this slot
running* has exactly that shape — host-written, server-read, moving while the run is going — so it
takes the same mechanism rather than a new one
([P-0085](../principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)).

## Decision

**`mcp::Slots` is a handle, and it is written where an aim is sent.**

```rust
pub struct Slots(Arc<RwLock<Vec<Pointed>>>);   // Pointed = (head, rest)

Slots::of(pairs)              // the deck as it stands
Slots::re_point(slot, &Aim)   // one slot is pointed somewhere else
```

- `crates/karakuri`'s `main` makes **one**, beside `opening` and for its reason, seeded from the
  working copies every watcher is about to be pointed at. `serve` gets a clone; so does every
  `Engine` the run opens, and through it every `Aiming`.
- **`Aiming` publishes, and nothing else does.** `Aiming::new`, `re_point` and `re_aim` each call
  `Aiming::publish`, which is `self.pointing.re_point(self.slot, &self.at)` and reads nothing but
  the aim. Those three are every occasion an aim leaves this program, so the published layout
  cannot be a step behind the watcher's.
- **The pair is derived from the aim inside `karakuri-environment`**, in the crate that owns both
  `watch::Aim` and `Slots`, so the walk from *where a watcher is pointed* to *the files behind a
  slot* is written once
  ([P-0087](../principles/0087-name-the-property-never-the-shape.md)).
- **One write per re-point.** `loading` writes several files and then sends one aim; the
  publication is that one moment, so a call arriving mid-load resolves against the layout before it
  or the layout after it and never half of either.
- `restored` no longer builds a `Slots`. It reads `Engine::pointing`, which is the same handle the
  server reads, so ADR-0308's workaround is **gone rather than kept beside the fix**.
- `karakuri-cli` calls `Slots::of` once and nothing else. It loads no Set mid-run — its only id is
  `--load-set`'s, settled before the deck is built — so the launch pairs are what that deck is
  running for the whole run, and the type is unchanged for it.

**The refusals are unchanged and are now about the right slot.** `Slots::path` walks the slot's
`kind` lines on every call and names what it found — *slot 1 holds one L4 and `index` is 1*, *slot 0
holds no L2: a deformation is optional* — so a node a load took away is refused naming the nodes the
deck holds **now**
([P-0083](../principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).

## Alternatives, and why they lost

### A fresh `Slots` rebuilt in `played` and swapped into the handle

The load's own report is where the panel already notices that a deck moved, and rebuilding the
whole layout there — four slots, one `Slots::of`, one swap — is fewer moving parts than a method on
`Aiming`.

**It loses at the second route, which is ADR-0304's finding said again one field along.** `played`
is the *load*. `wire_input` is not: a rewiring restates an aim through `Aiming::re_aim` and never
passes through `played`, and `App::resumed` builds a fresh set of watchers on the launch pair
without going near it either. So the publication would have to be written at three call sites and
remembered at the fourth, and the symptom of forgetting one is the defect this record is about —
silent, on the model-facing surface, and visible only to whoever compares a scratch file's name
against what a tool answered.

Putting it on `Aiming` costs one field and one line in two methods, and puts the write where the
value is already being read. There is nowhere left to forget it: an aim that leaves this program
without a publication would have to bypass the only two methods that can send one.

### `Slots` as a view over the host's aims, resolving through `Aiming::at` itself

The purest reading of *one derivation*: no publication at all, and the server reads the aims. It is
not reachable from here. `Vec<Aiming>` lives on `Engine`, behind a `Gfx` that needs a device, and a
`watch::Aim` is not `Clone` and holds an `mpsc::Sender` per slot; the server runs on threads that
have neither. Sharing it would mean a lock over the engine's own state taken from a connection
thread, which is a socket holding a lock the frame path wants — traded for a `PathBuf` clone this
surface pays once per call, on a path whose expensive step is a compile
([P-0091](../principles/0091-cost-is-known-before-it-is-paid.md) run the wrong way).

So the aim stays canonical and this is its publication, which is the second of the two shapes
P-0087 allows: *an identity is recorded, a pending state is derived every frame*. A published layout
is neither — it is a derived value with a reader on another thread, and what makes it safe is that
it is written from exactly one place and read nowhere else.

### Refuse a write while a slot is being loaded

The other family of answer: make the window explicit rather than closing it. A load would mark the
slot, the server would answer *deck B is taking a Set, try again*, and the mark would clear when the
build landed.

Refused on two counts. It answers the wrong question — the defect is not a *race*, it is a copy that
is wrong for the rest of the run, and a refusal that expires would leave the stale answer behind it.
And it is the price paid in the wrong currency: P-0094's ordering is refuse, then undo, then be
loud, and *refuse* is admissible where refusing is **free**. Here it is not free — it takes a
working surface away from a model at the moment an operator is changing material, which is the exact
shape of *buying the safety back from the operator* that rule's mirror clause rules out.

### Publish inside `Watch`, on the worker thread that receives the aim

The watcher is the thing that is actually pointed, so let it say so when it takes the re-point. It
loses on ownership: a `Watch` is per slot and knows its own paths, but the aim reaches it
asynchronously, so *published* would then mean *the worker has woken up* rather than *the operator
has loaded*. A model that wrote in that window would be answered about the layout before the load
with nothing to say it had happened. The host knows at the press; that is where it is said, which is
`Snapshots::record`'s own argument for taking the id rather than deducing it (ADR-0304).

## Consequences

- **`mcp::Slots` is `Arc<RwLock<Vec<Pointed>>>` and its field is private.** `Slots::of` replaces the
  tuple constructor at every site, `Slots::count` replaces `slots.0.len()`, and `Slots::unpointed`
  is the empty one a harness builds an engine with — it holds no row, so `re_point` writes nothing
  into it, and `serve` refuses a handle with no slots before it binds, so it cannot reach a client.
  `Slots::path` and the public `Slots::file` return an owned `PathBuf`, because what they read is
  behind a lock this must not hold past the call.
- **A poisoned lock is recovered rather than dropped**, which is where this parts company with
  `Opening::set`. What is behind the lock is a whole pair written in one assignment, and the safe
  fallback that makes `Opening` read as closed has no counterpart here: *the layout before the load*
  is precisely the wrong answer.
- **`crates/karakuri`: `Aiming` gained `pointing` and `slot`**, and is built through `Aiming::new`
  rather than as a literal. `Engine` gained `pointing`, `Engine::new` and `watched` each take it
  (both now carry `#[allow(clippy::too_many_arguments)]` with the reason at the signature), and
  `App` carries it for the same reason it carries `snapshots`: the server outlives every window the
  run remakes, so a handle rebuilt with the swapchain would leave it reading one nothing writes.
- **`restored`'s second `Slots` is deleted**, and ADR-0308's consequence naming it is annotated
  rather than rewritten — a record is what was decided then
  ([ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md)).
- **`karakuri-cli` is unchanged in behaviour** and says so at the one line it constructs: the
  comment states why this program never re-points, so the next reader does not add a publication it
  does not need.
- **Nothing about the frame path changed**, and no lock is taken on it. The publication happens on
  the thread that handles a press, and the server's read is on a connection thread.
- **Three tests hold it, each watched to fail against the layout taken at launch.**
  `mcp.rs`'s `a_re_point_moves_what_an_address_resolves_to` is over the wire on both tools — the
  read comes back `probe_l4_b` and the write lands in `after.kir` — and
  `a_node_a_load_took_away_is_refused_naming_what_the_slot_holds_now` is the refusal half.
  `crates/karakuri`'s `a_load_moves_what_the_mcp_server_resolves_against` is the CPU test that runs
  a real `loading` onto slot 1 and asserts the path the server would write to, the refusal for the
  renderer the loaded Set does not hold, and that deck A did not move. The existing `Slots` tests
  are unchanged apart from `Slots::of` and the owned path.
