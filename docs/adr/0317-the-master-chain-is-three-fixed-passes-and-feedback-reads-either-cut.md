---
id: 0317
title: The master chain is three fixed passes, and feedback reads either cut
status: accepted
date: 2026-09-09
supersedes: []
superseded_by: []
principles: [0064, 0085, 0090, 0091, 0092]
tags: [engine, console, vocabulary, store, manual]
---

> **Annotated 2026-09-10.** The deferral this record made — a writable L5 revives *"the day
> somebody writes the compositing down"* — is taken up by
> [ADR-0340](0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md): `kind L5`
> is specified under `docs/ir-spec.md`'s *Beyond* boundary, the chain becomes an ordered list of
> L5 slots, and the three passes here ship as L5 procedures. Two clauses of this record are
> superseded there — a pass at zero records no pass (a list spells *no pass* as *no slot*), and the
> four-field `master_chain` record. What stands: the three passes as written, the two feedback
> cuts, the linear-HDR placement before the tone map, and the measurements, which M5.16 re-takes.

# The master chain is three fixed passes, and feedback reads either cut

## Context

**The Master bay has drawn three effects since the mock's first commit and the engine has run
none of them.** `out` landed on 2026-08-30 with an operation, a record and a fader
([ADR-0224](0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)), and
that record paid a cost on purpose: with nothing between the mix's write and the tone mapper's
read, `out` and `exposure` were the same number in every frame this program could draw — measured
at zero bytes' difference, and held by a test its own record says *"is deleted the day the chain
has an effect in it"*.

Everything downstream of the bay was written to wait. `Operation::SetFeedback`, `SetBloom` and
`SetRgbShift` carried `Undecided`; `written` answered `Owed::Undecided` for all three;
[ADR-0227](0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md) declined to
name a chain's contents at all — *"a record kept for a thing that does not exist would be inventing
its contents"* — and `Record::MasterOut` said the same from the stream's side.

**Two questions were open, and the roadmap held both.**

- **Which of two things the chain is.** M5.8's *Blocked on* named it and said the file had named
  the wrong thing before: *"the blocker is which of those two the chain is, and nothing in the
  repository takes it"*. Three fixed passes between the composite's write and the present pass
  need no language change at all, on `crate::node::Merge`'s pattern; a writable L5 buys
  operator-written frame effects and closes the bay's `+ add` as well.
  [ir-spec.md](../ir-spec.md) states `L5`'s absence as **a condition rather than a principle** — a
  `kind` says what a procedure lowers to, the compositing is fixed, the one L5 that exists has no
  code to lower — and says what would end it: *"admitting frame effects is giving L5 a writable
  form, and a written form is code to lower."*
- **Which cut of the previous frame feedback reads.** In *The decisions nobody has taken*, with its
  cost argument already worked out: the previous frame is not one thing, *"a cut that is read has to
  be held, which is a frame-sized target and a copy per frame, so holding all of them against the
  chance that something reads one is the cost nobody would pay"*, and the shape it pointed at was
  that **the selection recomposes the pipeline**.

## Decision

The maintainer's, on 2026-09-09, in two parts.

### 1. The chain is fixed built-in presets

> 固定プリセット

**Fixed presets.** Three hand-written passes in the engine, between the composite's write and the
present pass, in the order the console draws them — **feedback, then bloom, then rgb shift** — each
with parameters the vocabulary can name. **No `L5` kind, no language change, and no `.kir` may
declare one of them.** `+ add` stays what it is: a note, because the arena the console's regions
live in has no insert and no remove.

`crates/karakuri-engine/src/master.rs` and `shaders/master.wgsl` are the whole of it — one shader
module with four fragment entry points over one bind group layout, which is what makes *these are
the chain's passes* checkable by reading one file.

### 2. Feedback reads either cut, and the parameter chooses

> 両方持って選択できるように

**Both, and selectable.** The two cuts are:

- **`mix`** — the frame as the mixer wrote it, before this chain touched it. **One echo and not a
  trail**: nothing read back has ever been fed back, so on still material it reaches `1 + amount`
  times the frame after exactly one frame and stops.
- **`exit`** — this chain's own output, after rgb shift and before the tone map. **A trail**,
  because what is read back already contains it: every layer is re-bloomed and re-fringed on each
  pass round the loop.

They are different pictures rather than different amounts of one, which is why this is a parameter
and not a choice taken here. **Only the chosen cut is retained** — the cost argument the roadmap
entry made, honoured rather than dodged: one frame-sized `Rgba16Float` target holds the history, and
the cut nothing reads is not copied.

### What is forced and what is chosen

**Forced.** That the chain runs in linear HDR, unclamped, upstream of the one tone map and the one
sRGB encode — [P-0064](../principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md),
and it is the reason the passes are on this side of the transport's tone map capsule. That every
pass writes the source's alpha through unchanged: alpha is coverage and none of these three changes
what a frame covers, which is `Deck::set_out`'s rule one pass upstream. That the retained frame is
part of the state a replay reproduces — [P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md)
— so it is a copy of a target this chain wrote, from a parameter a record carries, into a target
that reads as zero until it is written. That the targets are allocated at build and at resize and
never when a parameter moves — [P-0091](../principles/0091-cost-is-known-before-it-is-paid.md).
That the amounts are clamped where the record is applied and at no surface —
[P-0090](../principles/0090-a-surface-offers-it-never-decides.md).

**Chosen, and a wrong one here is a one-clause revision.** The **order** of the three: feedback
first because a trail should carry what the frame actually was, rgb shift last so the other two are
seen through it. **Bloom's knee at 1.0** and **bloom's radius at 1.2% of the frame's height**, both
fixed rather than controls — see *Alternatives rejected*. **RGB shift's reach**, 2% of the frame's
height at an amount of 1.0. **Feedback's ceiling at 0.95**, which is where a still frame settles at
twenty times itself instead of at infinity. **That every length is a fraction of the frame's height
and never a count of texels**, so one render scaled into two outputs
([ADR-0247](0247-one-frame-is-rendered-and-scaled-into-each-output.md)) is one picture at two
resolutions.

## Alternatives rejected

**A writable L5 — deferred, not refused.** It is the better answer to a different question: it buys
operator-written frame effects, closes `+ add`, and makes the deck count a property of a procedure
rather than of one built-in shader. It was not taken because **it is a language change bought to
draw three rows the console already draws**, and because the three passes wanted are the three
nobody would write differently. What would revive it is what `ir-spec.md` already names: **somebody
writing the compositing down**. The day an L5 has a form a `.kir` can declare, this chain becomes
three procedures shipped in `examples/` rather than three functions in `master.rs`, and nothing in
the vocabulary or the record has to move for that — `SetFeedback` names a pass, not a shader.
`ir-spec.md`'s paragraph is annotated rather than overturned: its condition still holds, because
these passes declare nothing and lower nothing.

**One retained cut, chosen here.** Either choice is defensible and neither is obviously right, which
is the argument against making it: `mix` is the honest one — the trail is of the material rather
than of itself — and `exit` is the one a VJ means by *feedback*. Picking one would have put a
picture out of reach for no saving, since what a second cut costs is **where the copy is recorded**
and not a second target.

**Retaining both, always.** Two frame-sized targets and two copies a frame — 14.06 MB of bandwidth
at 1280x720 — so that a parameter change costs nothing on the frame it happens. Refused on the
roadmap entry's own words: *"holding all of them against the chance that something reads one is the
cost nobody would pay."* The price of not paying it is one frame of discontinuity when the cut is
switched, which is a frame the operator asked for.

**Bloom's knee as a control.** It looks like the parameter a bloom most wants, and in a
display-referred pipeline it would be. Here 1.0 is not an arbitrary level: it is the top of the
range the sRGB encode is honest about, and everything above it is light the display cannot show. And
the control that decides how much of a frame is up there **already exists one pass upstream** — the
master out, which ADR-0224 put at the chain's entry precisely because the level before an effect is
part of the picture. A second knob would be a second way to say what `out` says.

**Bloom's radius as a control.** Refused on P-0091 rather than on taste: the tap count is what a
radius costs, so a radius a hand could turn either changes the number of fetches per pixel at
runtime — a cost discovered while it is on air — or spreads nine taps far enough apart that the
blur bands.

**A fourth parameter to `frame::compose`, with the chain owned by the program.** The chain is
owned by `Present` instead. `compose` has twenty call sites across three crates and the chain has
exactly `Present`'s lifetime and exactly `Present`'s size: it is four more frame-sized HDR targets
beside the one that module already owns and resizes, and putting them anywhere else is a second
thing to remember to resize. It does not weaken what that module claims — the chain adds no encode
and no transfer, so *sRGB is encoded once, here* is the same sentence it was.

**Growing `Record::MasterOut` to carry the chain.** That record's own documentation anticipated it —
*"it grows the day an effect lands in the chain"* — and the record beside it is the better shape,
for the reason ADR-0224 gives for not folding `out` into `Record::Look`: they are different values
in different passes. `MasterOut` is a **level**, ridden continuously by a fader; the chain is a set
of **settings**, moved by a press. Folded, a fader ride would rewrite four settings sixty times a
second and a settings change would rewrite the level a hand was holding.

**Three records, one per pass.** Refused on `Record::Look`'s argument at one more row: `feedback`
without `cut` beside it is not a picture anybody can reconstruct, and a stream that moved the bloom
without saying where the feedback stood would describe a chain a replay could not put back. One
record, written whole, completed from the chain that is running — which is what `Current::master_chain`
is for.

## Consequences

- **ADR-0224's tripwire fires, and that is the outcome it was written for.**
  `with_nothing_in_the_master_chain_the_two_levels_are_the_same_picture` is a test that record says
  *"is deleted the day the chain has an effect in it"*. It still passes, because a chain at zero
  records no pass at all — so the deletion is owed to a chain that is turned up rather than to a
  chain that exists, and the sentence that replaces it is
  `tests/master.rs`'s `a_chain_at_zero_is_the_frame_with_no_chain`. ✔ checked: the engine's test
  binary passes with the chain at zero and the two levels still indistinguishable there.
- **The default look is bit-identical.** With every amount at zero the mix writes straight into the
  target the present pass reads and no pass of this chain is recorded. ✔ checked, three ways in one
  test: a `Present` never told about a chain, one told a chain of zeros, and one turned up, run for
  two frames and put back — all three the same bytes.
- **What it costs, measured rather than predicted.** Four frame-sized `Rgba16Float` targets,
  allocated at build and at resize: **28.1 MB at 1280x720**, which is what a full deck of four slots
  costs. Per output texel and only for a pass that runs — feedback 2 loads and a multiply-add; bloom
  19 fetches over two passes; rgb shift 1 load and 2 samples; plus one frame-sized copy when
  feedback is on. ✔ measured by `examples/master_cost.rs` at 1280x720 on this machine (Metal,
  Apple M4 Pro, 2026-09-09), medians over 100 submissions with 20 discarded cold, each figure over
  an empty submission's own 0.087 ms: **feedback 0.38 ms** at the mix cut and **0.41 ms** at the
  exit cut, **bloom 0.80 ms**, **rgb shift 0.34 ms**, **all three 1.14 ms** — 6.8% of a 60 Hz
  frame. The chain's passes are timed as their own submission with the deck drawn outside the timed
  stretch, because a four-slot frame is 4.6 ms of GPU here and the chain would be inside its drift;
  host clock and `Device::poll` to a drained queue, biased high by the poll's round trip, which is
  `frame_cost.rs`'s reasoning verbatim (P-0095).
- **`karakuri-operation` loses three of its twelve `Undecided` payloads.** They carry `Feedback`,
  `Bloom` and `RgbShift`, and replacing the marker was a compile error at every construction site
  rather than a search — which is the whole of what `Undecided` was for. Nine remain, five of them
  the sequencer's. ✔ checked: the crate's own tests, and `panel_column.rs`'s scan.
- **Three panel badges move from `plan` to `has`** on [every operation](../manual/operations.html),
  which is M5.8's exit condition. ✔ checked by `panel_column.rs`, both directions.
- **`Record::MasterChain` joins the session vocabulary**, classified beside `Record::Look` and
  `Record::MasterOut` and dropped by the projection for their reason. The **library** half of
  ADR-0227 is untouched and still owed: no store directory is added, and what a chain saved under a
  name looks like is still for the record that has something to serialise. ✔ checked:
  `karakuri-store`'s projection tests, and `setfile.rs` refuses one in a Set file.
- **`docs/ir-spec.md`'s `L5` paragraph gains a statement and keeps its condition.** ✔ checked by
  reading: no `kind` was added, `parse.rs` is untouched, and `kind L5` is still refused.
- **The console's Master bay has five controls where it had one**, and `input::PROBES` says so. The
  mock's dash on the rgb shift row becomes `0.00`: the dash meant *a pass nobody has given a value*,
  which stopped being true when the chain existed. ✔ checked by `karakuri-console`'s own test files.
- **`Feedback::MAX` is written down twice**, in the vocabulary so a fader can be laid out and in the
  engine where the wall is. That is a convention rather than a structure, and it says so at both
  ends; ✔ what holds it is a test in `crates/karakuri`, the one package that depends on both.
- **`mix::change` gains both ends of the chain, and the second of them was already owed.** That
  decoder ends in a wildcard by design, and `Record::MasterOut` had been sitting under it since it
  landed: `crates/karakuri` reads the master records itself, so nothing noticed, and every path
  through the decoder — which is `karakuri-cli` live *and* on replay — put the level back at 1.0.
  A session that pulled the master out down replayed with it up. **That is a stream this program
  wrote and could not reproduce**, which is
  [P-0092](../principles/0092-the-same-inputs-produce-the-same-frame.md) exactly, and this record
  would have added a second one. So `Change::MasterOut` and `Change::MasterChain` are one arm each
  and both are here. ✔ checked by `mix`'s `both_ends_of_the_master_chain_decode_from_their_records`,
  and the cut's wire word is refused rather than defaulted on the tone map operator's terms —
  ✔ `a_record_this_build_cannot_obey_says_so_rather_than_vanishing`, because a default here would
  not report a wrong level, it would silently play one echo where the session had a trail.
- **`render::replay`'s driver returns three values where it returned two**, and the chain is the
  third for the look's reason exactly: a `master_chain` record moves it mid-session, so a run that
  took it as a parameter would replay every frame under whatever the stream started with. That is
  the failure the look's own paragraph in that file records having had. `render::to_sequence` is
  unchanged and passes `Chain::default()` — every amount at zero, no pass recorded — so a
  `--render` is bit for bit the frame it was.
- **One loop calls its driver before `compose` rather than inside the commit closure**, and it is
  the offscreen one alone. `Present::set_chain` decides what the mix writes into, and `compose`
  holds the `Present` by shared reference for the whole call, so a chain arriving from the commit
  closure arrives one statement after the decision it makes. What that costs on that path is
  nothing and it is provable rather than hoped: `PngSink::acquire` is `Ok(())` unconditionally and
  says so, so *a sink is never asked to answer for a frame already committed* cannot be violated
  by a sink that never refuses. Every loop with a sink that can refuse keeps the closure.
- **What the CLI still has no route to is a press.** No key on that surface names a pass of the
  chain, so `Live` writes no `Record::MasterChain` today; the reading is handed to `written` all the
  same, because what decides whether an operation can be answered is whether the reading was taken
  and not which surface asked. Handing `None` there compiles and no test on that surface catches
  it — ✔ checked by injecting it — and what it would produce is `Owed::NotRead` printed at the
  operation rather than a wrong picture, which is the loud answer rather than the silent one.
- **The feedback cut leaves *The decisions nobody has taken*.** What that entry predicted is what it
  turned out to be — the selection recomposes the pipeline — at the size of one
  `copy_texture_to_texture` recorded at a different point in the frame rather than a graph the
  language builds. Removing the entry is the roadmap's own pass and not this record's.
