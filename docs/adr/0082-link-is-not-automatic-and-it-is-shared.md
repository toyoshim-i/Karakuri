---
id: 0082
title: Link is not automatic, and it is shared
status: accepted
date: 2026-08-11
supersedes: []
superseded_by: []
principles: [0044]
tags: [signal, process]
---

# Link is not automatic, and it is shared

## Context

Link was built and then tested against rekordbox, and **the verification changed what it is for.**

rekordbox joins a Link session and its decks can be made to follow it, but **it never publishes the
deck's BPM**. Pioneer's own answer is that Link has no master mechanism, so a tempo fader cannot be
the thing that drives a session tempo. The picture therefore does not follow the music by itself.

My first conclusion was that this made Link **unusable**, and it was wrong twice.

## The two corrections

**Factually.** "Receive-only" was wrong. Pioneer's support documentation says the Link subscreen's
BPM *can* be driven from hardware with a MIDI mapping — a DDJ's encoder or its TAP button. It is
simply not connected to the deck's tempo; it is a separate act. So the operational picture is far
better than "re-match it by hand every track".

**And more importantly, by its own standards.** I was measuring it by **whether it works
unattended** — and that is another kind of software's criterion. It contradicted the position this
milestone had already taken **four times**: semi-automatic gain, the tempo octave, the NaN count,
the panic key. Every one of them is *show the measurement, a human moves it*, and **requiring a hand
is what this project chooses.** Calling it a failure in the one feature that arrived from outside
was incoherent, and I did not notice.

**The negative framing also hid the value that was there.** A tap on the Link subscreen **lands on
every peer in the session**; `b` taps a grid on this machine only. With a second machine or another
application present, a hand-set tempo that is *shared* is worth something — and "it needs a hand" had
erased it.

## Decision

The facts stay: rekordbox does not publish the deck BPM, the reason is that Link has no master, and
the picture does not follow by itself. The manual carries the three-step procedure that is otherwise
certain to be stumbled over — enable `[LINK]`, SYNC the playing deck, set the tempo **from the LINK
subscreen** and not from the fader.

**The conclusion changes from "unusable" to "not automatic, and shared."**

A design change I had said would be needed turned out not to be: Karakuri stays a **passive peer**
riding the tempo the room decided, because setting that tempo is properly the DJ's job.

## Evidence

Session 2026-08-11T16:52Z–17:10Z, commit `d82e9db`. Standing rule:
[P-0044](../principles/0044-doubt-a-conclusion-that-contradicts-a-position-already-taken.md).
