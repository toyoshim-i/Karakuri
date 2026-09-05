# A surface offers; it never decides

A control decides **what a press asks for**. It never decides **what may be asked**, and it does not
word the refusal it draws. A constraint on what the instrument will accept lives where the record is
applied, so every way in meets the same wall in the same sentence. A rule held in one surface binds
none: a second surface that never implements it does not fail, it just works differently for a year.

**What is published is what is shown, never what can be reached.** And what the engine fixes rather
than offers is a control that does not exist, so it fixes only what must never be asked for — a
correction the engine must apply whether or not a procedure asks belongs to the engine precisely
because a binding could be forgotten.

**What it rules out.** Access control dressed as presentation: if publishing gated reach, a Set's
author could lock the operator out of their own machine. A console refusing *go on air* because some
other slot's prime request is parked — in a live instrument, the one operation that cannot be
refused, refused from a rule the engine has never heard of — and a `bool` in the panel holding the
pending state, which goes stale the moment MIDI or MCP moves the request, fails silently and looks
like a slow panel. An operation that says *toggle*, *cycle* or *step*: a surface that can only step
has no way to arrive, and two surfaces stepping one control disagree about where they are. The price
of that is paid rather than avoided — a sixth copy of the blend modes, sync modes, residencies, tone
map operators, curves and wipe shapes, mirrored into the vocabulary rather than imported, because
importing `Residency` would make every surface that parses a MIDI map depend on `wgpu`. Four
spellings of one refusal, one per surface, each passing an assertion that the message *contains* "no
slot 9"; the fix is one `no_such_slot` and equality against it. A flag inventing a convenient syntax
with no record behind it: `--param 1:turbulence=2.6` was declined because there is nowhere in
`--set l1,l4` to put it, and inventing the spelling is accidentally designing the record stream. And
a Poisson spawn process, which bakes in an irregularity nobody can turn, when noise bound to
`spawn_rate` gives the same look under control.


**Where it holds.** [karakuri-operation](../../crates/karakuri-operation/) is the vocabulary, and its
`Cargo.toml` has no `[dependencies]` entries at all — `std` and no more — which is what lets every
surface depend on it and is why it owns the enumerations a destination is drawn from rather than
passing strings. Every variant names a destination: `SetStep` and not `ToggleStep`, with the reason
at the arm, and `SetResidency { deck, residency }` where two booleans were four combinations for
three states. The affordance is the surface's, and the blend chip's cycle lives in
`karakuri-console` and nothing at all in the vocabulary.
[`no_such_slot`](../../crates/karakuri-environment/src/lib.rs) is the one sentence the key handler,
the MCP tool, the MIDI router and the mix path all call, and the tests assert equality against it
rather than that a message contains a slot number; the engine's own `assert!` spellings are outside
that line on purpose, and the function's doc says so. [ir-spec.md](../ir-spec.md) is the authority
for the two halves that are not about a control: *What a Set publishes*, where an empty interface
publishes everything and a published range narrows without redefining, and *Spawn timing* and
*Binding noise*, where the engine owns the birth fraction because a procedure would forget it —
`l1.rs` substitutes `birth_frac * dt` for `dt` on an element's first pass and writes the fraction
back to 1.0, so the correction expires itself — while irregularity stays a `noise` object on a
`bind` with a kind, a rate in cycles per beat and a stream. `karakuri-cli`'s `Router` and `Surface`
are where a flag writes into the record rather than inventing one, and
[roadmap.md](../roadmap.md)'s *Settled decisions* says why. **What no code enforces yet is the
authority half**: no console control refuses anything today, so the first control anyone puts a lock
on is where this is first tested. Decided in
[ADR-0005](../adr/0005-spawn-is-an-accumulator-and-the-engine-owns-the-birth-fraction.md),
[ADR-0011](../adr/0011-a-noise-binding-carries-a-kind-and-a-rate-in-beats.md),
[ADR-0046](../adr/0046-a-flag-writes-into-the-record-it-does-not-invent-one.md),
[ADR-0100](../adr/0100-a-published-interface-is-a-choice-of-attention.md),
[ADR-0131](../adr/0131-one-refusal-sentence-per-mistake-across-the-surfaces-that-face-a-person.md),
[ADR-0180](../adr/0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md),
[ADR-0186](../adr/0186-one-operation-names-one-of-three-residencies.md) and
[ADR-0188](../adr/0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md).
