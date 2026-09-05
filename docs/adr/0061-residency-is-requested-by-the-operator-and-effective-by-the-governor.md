---
id: 0061
title: Residency is requested by the operator and effective by the governor
status: accepted
date: 2026-08-02
supersedes: []
superseded_by: []
principles: [0094]
tags: [engine, ui]
---

# Residency is requested by the operator and effective by the governor

## Context

The governor needs to move slots between residency levels. The operator also sets them. One field
cannot hold both without one silently overwriting the other.

## Decision

Two fields. **Requested** is the operator's and only the operator writes it. **Effective** is
recomputed by the governor on every pass. A slot whose requested level is Priming and whose effective
level is Allocated is **parked**: the request stands, and it starts on its own when room appears.

This is what lets [P-0094](../principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md) hold
without the governor having to remember anything: it never destroys an intention, it only declines to
grant one right now.

## Consequences

- **A parked slot must be visibly different from one the operator turned off.** They read identically
  in the status line, and the operator cannot tell "I did this" from "the machine could not". Logged
  as owed work.
- The engine also owes `measure_slots` at startup — without it an unmeasured Live slot suspends
  priming by default, which is the safe direction and the wrong experience.

## Evidence

Session 2026-08-02T04:19Z.
