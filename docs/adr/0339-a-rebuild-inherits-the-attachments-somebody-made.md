---
id: 0339
title: A rebuild inherits the attachments somebody made
status: accepted
date: 2026-09-10
supersedes: []
superseded_by: []
principles: [0084, 0087, 0092]
tags: [engine, binding, swap, audio, m5]
---

# A rebuild inherits the attachments somebody made

## Context

The maintainer, 2026-09-10:

> consoleで画像が入力音声に反応しなくなってる気がする。cliでは出来てたので、何かが壊れてるかも。

The panel's picture stopped answering to the room; `karakuri-cli --audio-in default` on the same
material did not.

**The audio path itself is intact, end to end, and was checked before anything was changed.**
`karakuri_environment::audio::Audio::frame` reads the device, writes `Record::Audio`, reads it back
and installs it with `Signals::set_audio` — driven live against this machine's default input it
answers `energy` at confidence 1.0 on 597 of 600 frames. The panel writes that measurement into the
deck's own signals once a frame, unconditionally, before the frame composites — `measure_audio` in
`crates/karakuri/src/main.rs`, which is `karakuri-cli`'s three lines with the recorder and the tempo
source removed. `watch::Aim::bindings` is restated by every re-point and `swap::Request::bindings`
by every rebuild, with no `..` on either destructuring. `setfile::load`, `resolve` and `unbundle`
all carry a `bind` record through. `Set::prepare` samples the bus, blends on confidence and writes
the uniform, and `karakuri-engine`'s own suite asserts that at the texel.

What is left is the one thing [ADR-0319](0319-an-attachment-is-a-session-record-and-taking-a-parameter-back-removes-it.md)
named as open when it built the control:

> A live attachment and a live grant do not survive a re-point, and an attachment does not survive a
> rebuild. `swap::Request::bindings` and `Request::authorities` are restated from `Watch`, which
> only a re-point writes — so a save of any `.kir` in a slot walks a live attachment back, exactly
> as it walked a ride back before ADR-0282.

**On the panel that is the whole of its audio reactivity.** A run with no paths opens on a *pair* of
`.kir` files rather than on a Set file ([ADR-0271](0271-the-panel-opens-on-the-demo-rather-than-on-the-reference-workloads-pair.md)),
and a pair carries no `bind` — `examples/star_vortex.kset`'s two binding records, `energy` onto the
funnel's `scale` and `band0` onto its `ripple`, are in the file and not in the pair. So the two ways
a panel's picture answers to sound are loading that Set off the Library bay, and attaching a signal
by hand on the Inspector's sensitivity row — the control ADR-0319 landed the day before this report.
The second worked until the slot rebuilt, and then silently stopped. `karakuri-cli` has no such
control and takes its attachments from `--bind` and `--load-set`, which are in every request it
makes, so the same material there went on answering.

ADR-0319 declined to close it, on ADR-0280 §6's terms — *it is a change to what a rebuild is*. That
change is [ADR-0282](0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md),
and it has been made. What is left is to read the same sentence about the other writer.

## Decision

### 1. The rule, and it is ADR-0282's read one writer along

*A binding the request stated comes from the request, and one anything else attached carries.*

`Set::carry_bound_from(&Set)`, called in `HotSwap::install_if_ready` immediately after
`Set::carry_moved_from` — the one moment both Sets are in one hand. For every binding the outgoing
Set holds, at the `(layer, index, key)` address `Set::bind` keys *at most one binding per address*
on: attach it to the incoming Set unless the incoming Set already holds one at that address.

An operator who attaches `energy` to a parameter keeps the attachment across a save of the `.kir`,
for the reason an operator riding `exposure` keeps their hand on it.

### 2. There is no `Set::moved` for bindings, and there does not need to be

ADR-0282 had to add a field, because `params` holds one number per key and a number carries no
memory of whether the code or a hand put it there. Bindings have no such ambiguity: **a `.kir`
cannot bind.** The signal bus is not readable from IR
([ADR-0251](0251-the-signal-bus-is-not-readable-from-ir-and-beats-is-the-one-exception.md)), so a
procedure declares parameters and never sources, and every binding a freshly built Set holds came in
on `Request::bindings`. The address is therefore the whole of the discrimination: what the request
named is this build's answer, and what it did not name is somebody's attachment.

That is one derivation and no new state
([P-0087](../principles/0087-name-the-property-never-the-shape.md), whose fix is stated
as *there is exactly one derivation of X*).

### 3. A binding the request states still wins at its own address

The clause that separates a rebuild from a load, restated. A slot pointed at a Set file states that
file's `bind` records, and an operator who loads a preset over a slot they had attached a signal on
asked for the preset. Without the clause the load comes up wearing the attachment.

### 4. Not in `HotSwap::install`

`install` is the replay path. A replay builds afresh from the record stream and meets each
`Record::Source` at the frame it was made at (ADR-0319, ADR-0280 §7), so inheriting there would be a
second, unrecorded source of the same attachment and P-0092 would stop holding.

### 5. What the rule answers, case by case

- **A key this build no longer declares.** `Set::bind` refuses it — `Bound::NoSuchParam` — and the
  attachment is passed over in silence, exactly as a carried value that lands nowhere is. The author
  deleted the `param` line in the file that caused the rebuild; the count `carry_bound_from` returns
  is what a caller with something to say would say it with.
- **A wildcard attachment and an addressed one.** Two attachments, at two addresses, and a request
  stating one of them carries the other. That is `Set::bind`'s own key and `Set::unbind`'s.
- **A take-back does not carry.** `Set::unbind` on a key the request states puts the request's
  binding back at the next rebuild, because the request's statement is this build's answer for that
  address. This is the same asymmetry the values have — a ride carries and *riding a knob back to
  where the file put it* is still a ride — and it is named rather than closed: closing it would mean
  a Set remembering an absence, which is the fourth state ADR-0319 refused.
- **A `control:` source.** Carried like any other, and `Set::bind` checks it against the incoming
  Set's interface — `Request::published` is restated, so the control is already there when this
  runs. A rebuild that stopped publishing the control refuses the attachment and leaves the
  parameter at its own value, which is what a source that is not there has always meant.
- **A node added or removed.** The carry is addressed, so a bare key can never re-land on a
  different node and a node this build no longer has is passed over — `Request::live` and
  `Request::authorities`' terms exactly.
- **An authority granted live is still not carried.** `Deck::set_authority` has the identical hole
  and is deliberately left, because it has no symptom: nothing in this workspace writes a parameter
  on an agent's behalf, so a level reaches only `Set::write_param`'s wildcard refusal. It is one
  more line at the same call site on the day something does.

## Alternatives

### a. Write the attachment back into `Watch::bindings` from the frame loop

The obvious shape: a press attaches to the live Set *and* tells that slot's watcher, so the next
request states it and nothing has to be inherited. Rejected for the reason ADR-0282 rejected the
same move for values, sharpened: it puts the render thread on the far side of a channel into the
build worker for every press, and it makes a request depend on what happens to be live — which is
precisely the property every field of `Request` is documented against. A request has to be
reproducible from a record stream; an attachment made two seconds ago is not in the stream the
watcher is reading.

### b. Give `Binding` a "stated by the request" flag

ADR-0282's `Set::moved` transplanted. Rejected by §2: it would be a second answer to a question the
address already answers, and a Set fresh out of `build_many` would have to mark every binding it was
handed. Same cost, one more thing to keep in step.

### c. Leave it, and have the console re-attach after a swap

The console reads `Set::bindings` per frame already, so it could remember what it attached and put
it back on `Event::Swapped`. Rejected: a surface holding state the engine is the model of record for
is the seam ADR-0156 draws, and the gap between the swap and the repair is a frame of picture with
the attachment silently gone. It also answers only for attachments *this* surface made, where the
hole is the engine's.

### d. Restate the attachments from the session stream

`Record::Source` is written for every attachment, so a rebuild could replay the slot's sources.
Rejected: it makes a rebuild depend on a recording that may not be running — the panel writes a
session only when `rec` is armed — and a rule that holds only while somebody is recording is not a
rule.

## Consequences

- **A save of a `.kir` no longer walks an attachment back**, so the panel's picture goes on
  answering to the room across a rebuild. That is the report this record closes.
- **A load still beats an attachment**, by §3, so nothing about loading a preset moves.
- **A frame that swaps clones the outgoing Set's bindings**, once per swap, on the frame that is
  already reallocating render targets. `Set::bind` allocates nothing beyond the push for a signal
  name that is not a `control:` — and it is the same allocation budget `carry_moved_from` is
  documented under one line above (P-0091).
- **A bare panel run still shows no bound material**, and that is ADR-0271's cost rather than this
  record's: the pair carries no `bind`. Loading `star_vortex` off the Library bay, or attaching a
  signal by hand, is where a panel's picture answers to sound — and both now survive a rebuild.

## What was watched fail

`binding::gpu::an_attachment_made_live_is_still_driving_after_the_next_rebuild`, run against the
tree as it stood, prints

```
assertion `left == right` failed: the rebuild left the slot with no attachment at all: what an
operator attached to `radius` was walked back by a save that states no binding
  left: 0
 right: 1
```

and against the same tree with the `carry_bound_from` call removed from
`HotSwap::install_if_ready` — the injected defect — it prints the same thing. With the call in
place it passes: a silent room and a loud one draw two different pictures four frames after the
rebuild lands, and `radius` reads `0.5` and `8.0` rather than the `2.5` its declaration holds.
