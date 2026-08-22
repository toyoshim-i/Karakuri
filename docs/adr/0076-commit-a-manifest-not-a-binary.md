---
id: 0076
title: Commit a manifest, not a binary, and never fetch from build.rs
status: accepted
date: 2026-08-11
supersedes: []
superseded_by: []
principles: [0006]
tags: [process, docs]
---

# Commit a manifest, not a binary, and never fetch from `build.rs`

## Context

The proposal was to import plugin sources as a submodule and **commit the built shared library** to
the main repository.

## Decision

**Commit a manifest instead.** `plugins.toml` carries, per plugin: a name, an ABI version, the
source repository's tag, and **per target triple** an asset name and a sha256.
`cargo xtask plugins` fetches only your own triple, verifies the checksum, and puts it in a
gitignored directory — **which is also the default `--plugin-dir`**, so the development flow and the
installed flow use the same search path and "works while developing, missing once distributed"
cannot happen structurally.

Committing the bytes was rejected on three grounds: macOS distribution needs signing and
notarisation, universal-versus-per-arch becomes a question, and above all **code nobody in that
repository can review would live in its git history for ever**.

## The absolute rule: not in `build.rs`

The moment `cargo build` touches the network, everything this was buying is gone. A fetch inside a
build script **breaks offline and sandboxed builds** and runs for people who do not want the plugin.
An xtask is explicit, and — the part that matters — **a failed fetch means the feature is absent,
not that the build is broken.** That escape exists only because the plugin is optional, and an
ordinary vendored dependency does not have it.

## Consequences

**The manifest earns its keep a second way, which is the stronger one.** Absence has three states,
and only a manifest can tell the first two apart:

| State | What the host should say |
| --- | --- |
| No entry for this triple | The feature does not exist on this platform |
| Listed but not fetched | Run `cargo xtask plugins` |
| Present but the ABI disagrees | Refused at the handshake, with both versions named |

Without it, all three collapse into "file not found", and a Windows user cannot tell whether Syphon
is impossible or merely unfetched. That is the third notch on the same distinction as
[ADR-0074](0074-a-plugin-boundary-is-drawn-by-the-deterministic-path.md)'s point about *why*
something is missing.

The main repository's CI never touches the network; per-target builds and publishing belong to the
plugin's own CI.

## Evidence

Session 2026-08-11T08:59Z–09:01Z, `docs/plugins.md`.
