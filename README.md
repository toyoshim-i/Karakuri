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
  procedure, rewrite it, and be told what the compiler and the frame budget made of it.
  What has actually been demonstrated is in [docs/roadmap.md](docs/roadmap.md)
- **Real-time.** A changed procedure is compiled on a worker thread, installed at a frame
  boundary, watched for a window, and rolled back automatically if it costs too much.
  Nothing allocates or compiles a shader on the render thread — that and the rest of the
  rules in force are [docs/invariants.md](docs/invariants.md)
- **For VJ work.** Up to four Sets on a deck, each with its own gain, opacity and blend
  mode, on a beat grid the room's tempo can drive. How to play it is
  [docs/manual.md](docs/manual.md)

---

## Quickstart

Rust stable and a GPU.

```sh
cargo run -p karakuri-cli                                    # a window
cargo run -p karakuri-cli -- --watch                         # edit a .kir, watch it swap
cargo run -p karakuri-cli -- --set a.kir,b.kir --set c.kir,d.kir   # two Sets, mixed
cargo run -p karakuri-cli -- --audio-in default --bpm 128    # play to the room
cargo run -p karakuri-cli -- --render out.png --frames 240   # one frame to a PNG
```

Press `h` in the window for the keys and `s` for the status line.

**To play it rather than to judge it, read [`docs/manual.md`](docs/manual.md)** — a
walkthrough, every flag and key, what the status line means, and a list of the things it
deliberately will not do. That last section exists because this document is not where an
operator would find them: "the downbeat is arbitrary until Link" is a fact about playing,
not about design, and it had no home until the manual had one.

---

## Stack

- Rust + wgpu 26 (WGSL), winit
- No UI for now. V1 is a CLI plus a window, and **the CLI is scaffolding rather than the
  destination** — the end state is a GUI application, so the command line is deliberately an
  auxiliary way to reach what the records already carry
- Audio input exists — spectrum, energy, onset, and a beat grid that corrects the local
  oscillator. External sync exists out of process: `--tempo-source` runs a separate program
  that reports where the beat is, and the first one is Ableton Link. See [docs/plugins.md](docs/plugins.md)

---

## Where to go next

- **[docs/contributing.md](docs/contributing.md)** — how this repository is run: the working
  style, the gates that a commit and a push have to pass, the test commands, and where
  `docs/adr/` and `docs/principles/` fit. It is the file to read before changing anything
  here
- **[docs/manual/](docs/manual/)** — the manual as published, at
  <https://toyoshim-i.github.io/Karakuri/manual/>, for somebody who wants to play the
  instrument rather than build it
