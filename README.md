# Karakuri

A GPU-native, AI-native real-time visual generation system for VJ work.

Procedural generation centred on primitive drawing, executed on the GPU, where the
procedures themselves are generated from prompts, saved, and recalled. The design goal is
that manual human control and autonomous AI control coexist on the same mechanism.

Four properties define it, and each has a document behind it:

- **GPU-native.** `.kir` files go in — at least one geometry and one renderer, and
  optionally deformations, a camera and a field — and the CLI puts them through parsing,
  type and contract checking, cost estimation, WGSL generation, and pipeline creation.
  There is no hand-written shader anywhere in that path. The language is
  [docs/ir-spec.md](docs/ir-spec.md)
- **AI-native.** An LLM can write this language from the specification alone, and `--mcp`
  serves the Model Context Protocol on loopback so a chat client can read a slot's
  procedure, rewrite it, wire what that procedure declares to another node, and be told what the
  compiler and the frame budget made of it.
  What has actually been demonstrated is in [docs/roadmap.md](docs/roadmap.md)
- **Real-time.** A changed procedure is compiled on a worker thread, installed at a frame
  boundary, watched for a window, and rolled back automatically if it costs too much.
  Nothing allocates or compiles a shader on the render thread — that and the rest of the
  rules in force are [docs/principles/](docs/principles/), one file each
- **For VJ work.** Up to four Sets on a deck, each with its own gain, opacity and blend
  mode, on a beat grid the room's tempo can drive. How to play it is
  [docs/manual.md](docs/manual.md)

---

## Quickstart

Rust stable and a GPU.

```sh
cargo run -p karakuri                                        # the console
cargo run -p karakuri -- geometry.kir renderer.kir           # the console, on your own pair
cargo run -p karakuri -- --presets DIR --store DIR           # tell it where its data lives

cargo run -p karakuri-cli                                    # a window
cargo run -p karakuri-cli -- --watch                         # edit a .kir, watch it swap
cargo run -p karakuri-cli -- --set a.kir,b.kir --set c.kir,d.kir   # two Sets, mixed
cargo run -p karakuri-cli -- --audio-in default --bpm 128    # play to the room
cargo run -p karakuri-cli -- --render out.png --frames 240   # one frame to a PNG
```

`karakuri` is the panel and `karakuri-cli` is still what you play a whole set with: the
console has the picture, the deck previews, the transport, the mixer, the Library bay and
the Inspector, and no MIDI, MCP, replay or session recording yet — the four `mcp` pills that
open a class of operations to a model are drawn and pressable, and nothing in this process
serves MCP for them to govern. **It listens to the room**: it opens the
default audio input at startup, the transport row's `audio-in` pill says which one and
lists the others, `b`, `,` and `.` tap the beat and move the grid an octave, and `o` and `p`
nudge the latency offset. **Every slot
watches its `.kir` pair**, so the Staging lane is empty until a file changes and then carries a
row per slot whose newest build has a verdict outstanding. **And it keeps an arrangement**: the
transport's arrangement pill names the layout you are working in and puts it back later, under
`arrangements/<name>.arrangement.json` in the store — the one thing this program writes to disk
on purpose. The Library lists what your store holds and what ships with the
program, and `l` puts the row under the cursor on the selected deck. The Master bay draws its out row and
the Sequencer bay draws nothing but its head. Press `h` in the CLI's window for the keys and `s`
for the status line.

**To play it rather than to judge it, read [`docs/manual.md`](docs/manual.md)** — a
walkthrough, every flag and key, what the status line means, and a list of the things it
deliberately will not do. That last section exists because this document is not where an
operator would find them: "the downbeat is arbitrary until Link" is a fact about playing,
not about design, and it had no home until the manual had one.

---

## Stack

- Rust + wgpu 30 (WGSL), winit
- **The CLI is scaffolding rather than the destination** — the end state is a GUI application, so
  the command line is deliberately an auxiliary way to reach what the records already carry. That
  application is being built: `cargo run -p karakuri` — the binary in
  [`crates/karakuri`](crates/karakuri) — opens a window with
  [`karakuri-console`](crates/karakuri-console)'s arrangement in it — dividers that drag, regions
  that fold, a live engine frame in the Program bay, the deck previews under or beside it, the
  transport, the mixer read off the deck, the Library bay
  listing what the store holds, the Inspector read off the running Set, and the outputs. **It is a program now**, and the console crate is
  not one: it holds the arrangement and the view and opens nothing, because the window, the
  device and the event loop are the binary's (ADR-0156). The binary is what the panel column of
  [the operations page](docs/manual/operations.html) is measured against (ADR-0213). It was an
  example until ADR-0214, and for one reason: everything a program needs beyond the panel was in
  `karakuri-cli`, which has no library target, so there was nothing for a binary to sit on. There
  is now — [`karakuri-environment`](crates/karakuri-environment). `karakuri-cli` is still what you
  play a whole set with: the console has no MIDI, MCP, replay or session recording yet, and it does
  open an audio input — so the signal bus carries a measurement rather than an invention, and the grid follows
  the room
- **Every operation is named once and every surface routes into that name** — the manual's first
  rule. [`karakuri-operation`](crates/karakuri-operation) is every one of those names, checked against
  [the manual's own page](docs/manual/operations.html) by a test — **how many there are is not written
  down in this repository**, because a transcribed total goes stale in silence every time the page
  gains a row; `grep -c '<h3' docs/manual/operations.html` is the count. And
  [`karakuri-operation-record`](crates/karakuri-operation-record) is where one becomes a record —
  the step that makes a fader, a key and a MIDI knob the same thing. **The MIDI map is an
  `Operation` now** (ADR-0196), and fifteen of the CLI's thirty-nine keys reach the deck through
  the same call it does (ADR-0198); of the rest, nine name an operation whose record nobody can
  write yet and twelve name one that writes no record at all and therefore has to be performed by
  the surface holding the state. **All four surfaces have an answer now**: the console's own
  arrangement operations stay where they are, blocked on the manual rather than on the code
  (ADR-0197), and **MCP names its seven tools' operations and performs them itself** (ADR-0199) —
  every one of the seven writes no record where it is asked, so there is nothing for `Live::operate`
  to do with them, and what routes is the name. **A model is connected to every operation and the
  ones that could stop a show are refused until an operator opens their class** from the head of the
  bay it belongs to (ADR-0235), which is a setting of that same map rather than an operation of its
  own (ADR-0236). The manual's MCP column is the first of the four
  route columns a test can check
- Audio input exists — spectrum, energy, onset, and a beat grid that corrects the local
  oscillator. External sync exists out of process: `--tempo-source` runs a separate program
  that reports where the beat is, and the first one is Ableton Link. See [docs/plugins.md](docs/plugins.md)

---

## Where to go next

- **[docs/contributing.md](docs/contributing.md)** — **Every contributor and coding agent MUST read this first**: how this repository is run, the standing invariants in [`docs/principles/`](docs/principles/), and the ADR lifecycle ([Principle 0066](docs/principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md)). It is the essential file to read before designing or implementing any change.
- **[docs/roadmap.md](docs/roadmap.md)** — **what exists today, what is next, and the decisions it is waiting on.** *What exists today* is the status page, part by part, and *Where this goes next* at the end of it is the order and the open questions. Read it after contributing.md if you are picking the work up rather than looking something up.
- **[docs/manual/](docs/manual/)** — the manual as published, at <https://toyoshim-i.github.io/Karakuri/manual/>, for somebody who wants to play the instrument rather than build it. It is also **the specification the console is built to**: the panel is checked against it, and where the two disagree the manual is what changes last.
