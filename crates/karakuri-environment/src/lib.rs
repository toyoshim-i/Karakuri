//! The program: everything this instrument deals with that is not itself.
//!
//! **A module belongs here if what it deals with lives outside this process —
//! a disk, a device, a port, a socket, another process — or is the record of
//! what happened.** That sentence is the charter, and it is a test rather than
//! a description: the next module anyone proposes for this package is checked
//! against it, and the answer is not a vote. It is stated in
//! `docs/adr/0215-the-package-is-karakuri-environment-and-a-module-belongs-if-what-it-deals-with-is-outside-this-process.md`,
//! which is also where the name comes from and where the five nouns it was
//! weighed against are measured.
//!
//! The second half of the sentence is not decoration. Two of the modules in
//! scope — the metadata card and the mixer's wire spellings — touch no device
//! and open no file, and are entirely the record of what happened; a test with
//! only the first clause would have left them behind in the command line,
//! which is wrong. Both clauses are load-bearing and neither stands in for the
//! other.
//!
//! ## Why this is a package and not a module of the binary
//!
//! `docs/adr/0214-the-program-moves-out-of-the-cli-and-two-thin-binaries-sit-over-it.md`
//! is the decision, and its argument is a bill that had already been paid five
//! times. `karakuri-cli` has no library target, so nothing in this workspace
//! could depend on any of this: the frame loop was written twice, an operation
//! record needed a whole new crate to land in, the Library bay's time column
//! was **cut** rather than draw a third date format, and the console's example
//! transcribes a store path and a residency parser by hand. Each of those
//! arrived looking like a local question. None of them was.
//!
//! So: **two thin binaries sit over this package.** `karakuri-cli` is one
//! today — it keeps its flags, its terminal, its keys and its `--headless`
//! runs, and it parses arguments and calls in here. The panel is the second
//! when it exists, and it is the destination; `README.md`'s sentence that the
//! CLI is scaffolding rather than the destination is unchanged by any of this.
//! Nothing in this crate is the command line's and nothing in it is the
//! panel's. Both are surfaces, which is what the vocabulary has said since
//! `docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md`.
//!
//! What follows from that is the rule to hold when adding to this crate: **no
//! module here may know which surface it is under.** A window, an event loop
//! and a key handler are a surface's; the disk, the ports and the record are
//! this package's, and the day a module here needs to ask which binary called
//! it is the day the boundary has been drawn in the wrong place.
//!
//! ## What is here, and what is not yet
//!
//! ADR-0215 applied its test to thirteen modules and thirteen passed. This
//! crate holds nine of them — [`audio`], [`compile`], [`history`], [`meta`],
//! [`render`], [`scratch`], [`session`], [`setfile`] and [`tempo_source`] —
//! because ADR-0214 left open whether the move lands in one commit or several
//! and answered its own question with *several is the likelier*. The seven that
//! came first were the seven that named nothing else in `karakuri-cli`; the
//! metadata card and the Set file came second, and they brought the two items
//! they named with them — the card writer that puts a card in a store, and a
//! Set file's per-layer node names, which are part of that file's shape. Each
//! slice is a package boundary and not a redesign: `use` paths changed, `pub`
//! appeared where crate-private had been enough, and the code inside the
//! functions did not.
//!
//! The watcher, the MCP server, the mixer and the MIDI map are still in
//! `karakuri-cli` and still pass the test. They are owed the same move. So is
//! `Clock`, which ADR-0214 refused to guess at and ADR-0215 settled: wall-clock
//! time comes from outside this process, so it belongs here — while `Live`,
//! which holds a window and a device, does not and stays with the surface.
//!
//! **These are one-line restatements; the canonical text is one file each in
//! `docs/principles/` and one record each in `docs/adr/`, and where this
//! comment disagrees with them, this comment is the one that is wrong.**

// Each module's own header is the documentation for it, and there is
// deliberately no second sentence here: a `///` on one of these declarations
// would be a doc fragment written in *this* file's scope, and rustdoc resolves
// a module's `//!` links in whichever scope the fragment came from — which
// silently unresolved six working links the first time this list was
// written. A `//` comment such as this one is not a fragment and costs nothing.
//
// - `audio` — a microphone, the beat it is tracking, and the record for both.
// - `compile` — a `.kir` off a disk, through the pipeline, with its bytes kept.
// - `history` — the edit history: a directory per day, a chain per procedure.
// - `meta` — an artifact's card: what a compile pass can say, and where it lands.
// - `render` — a frame written to a PNG: the window's path, minus the window.
// - `scratch` — the copies a live run edits, so an original is untouched.
// - `session` — the recorder that writes the stream and the split that reads it.
// - `setfile` — the material as a record: what a Set was, written down and read back.
// - `tempo_source` — another program's clock, and what to believe of it.
pub mod audio;
pub mod compile;
pub mod history;
pub mod meta;
pub mod render;
pub mod scratch;
pub mod session;
pub mod setfile;
pub mod tempo_source;
