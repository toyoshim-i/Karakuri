---
id: 0081
title: ndjson over a pipe, and not protobuf
status: accepted
date: 2026-08-11
supersedes: []
superseded_by: []
principles: [0085]
tags: [format, process]
---

# ndjson over a pipe, and not protobuf

## Context

Two objections, both fair on their face: **ndjson for time-critical IPC is extravagant**, and
**standard I/O on POSIX drags a TTY in**.

## Decision

**A pipe carrying ndjson**, and the reasoning is worth keeping because most of it is about which
things do not matter.

**The TTY does not enter.** A child spawned with `Stdio::piped()` has a **pipe fd**; no termios, no
line discipline. TTY cost applies when stdout is a terminal, which is only while a human is
debugging with `cat`.

**"Time-critical" does not hold**, and this is the substance. What crosses is a **timestamped
anchor** — `{bpm, beat, at_clock_us, helper_clock_now_us}` — evaluated from `at_clock_us` and **not
from arrival time**. So transport delay does **not** become phase error: arriving 10 ms late, it is
still read as a value tied to a moment 10 ms ago. Jitter likewise changes nothing. **Making the
transport faster improves nothing measurable** — and optimising where it cannot matter is the
genuinely extravagant move. (Clock alignment is one subtraction on receipt; the error is one-way
transit on a local pipe, tens of microseconds, against a latency offset that steps in 5 ms — two
orders below. No round trip needed.)

**And it is not on the frame path.** A few messages a second, read by a reader thread that parks the
latest anchor while the frame reads without blocking — the same shape audio already has.

**Pipe versus socket is decided by process lifecycle, not by encoding.** With a spawned child, EOF
says it died and `try_wait()` gives the exit code; a socket means cleaning up files, stale sockets
and reconnection written by hand. Bidirectional, if ever needed, is stdin.

## Protobuf, rejected

A genuine candidate — a `.proto` is a far better artefact for a public interface than "read our Rust
structs". Rejected on two grounds:

- **`protoc` is the very thing that started this.** The motivation for the whole design was not
  wanting the toolchain to widen. Generated code can be committed and `protoc` can be vendored, but
  either way a moving part enters a workspace that closes over `cargo` today. **Designing a separate
  process to avoid cmake, and then adding protoc for its communication, is out of order.**
- **It raises the bar for third-party implementations rather than lowering it.** The point of a
  general interface is that anyone can write a tempo source. A JSON line is one `printf` in any
  language, with no library — a shell script can emit it. Five scalars a few times a second do not
  repay a protobuf toolchain on the implementer's side.

**Self-describing wins because version skew is normal, not exceptional**: the helper is a separate
repository on a separate release cadence. Unknown fields are ignored, so an old helper and a new host
**degrade instead of breaking** — the position this project already takes with unknown record tags.

## An argument I withdrew

I had argued that ndjson lets you run the helper alone and `cat` its output before a show. **In the
product nobody types the command** — the main application spawns it and indicates in the GUI whether
it is alive. That was **developer convenience dressed as a design rationale**, and withdrawing it
left most of the ndjson case standing on the points above instead.

It also added a requirement I had missed: **the protocol must carry state, not only data.** Is the
helper alive; is Link connected and to how many peers; and **which source the tempo is coming from
right now** — Link, the tracker, or `--bpm`. The third matters most, because what an operator needs
to see is not the bpm but where the bpm is coming from.

## Evidence

Session 2026-08-11T14:35Z–14:42Z.
