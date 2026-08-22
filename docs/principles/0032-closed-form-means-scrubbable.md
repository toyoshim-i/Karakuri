# Closed form means scrubbable

A procedure that never reads an attribute it emits, never spawns and never kills is **closed form**:
its picture at any `t` is a pure function of `t`. Forward at any rate, held, and **backwards** are all
free. Not needing to be warmed is the smaller half of that; **being scrubbable is the larger one.**

Accumulating material is not barred from scrubbing — it is **priced**. Running back to a target time
means re-running from zero, and because this engine is bit-exact that reproduces **the same state**,
not an approximation. Everything paid for determinism buys this. So the operator's question is not
*can this be scrubbed* but **will this scrub arrive in time**, which the budget machinery answers.

**What it rules out.** Claiming closed form loosely. The classifier produces **no diagnostic**, so it
can only be wrong silently: strict-wrong costs an unneeded warm-up, while permissive-wrong puts a slot
on air unwarmed *while telling the governor it needed no warming*, and under transport it is a scrub
producing garbage. Spawning disqualifies even when `element` is perfectly pure, because *whether an
element exists* is accumulated engine state — jumping to `t = 30` gives one frame of newborns, not
thirty seconds of population.

**Necessary for a seek, not sufficient.** A seek also needs everything else that is a function of time
to be evaluable there, and an oscillator under tempo correction has a past phase that depends on the
correction history.

**Where it holds.** [ir-spec.md](../ir-spec.md), "Closed form versus accumulating"; `is_closed_form`
in [karakuri-ir](../../crates/karakuri-ir). Decided in
[ADR-0028](../adr/0028-closed-form-and-accumulating-is-a-static-classification.md) and
[ADR-0058](../adr/0058-closed-form-is-worth-more-for-scrubbing-than-for-priming.md). Replaces the
retired P-0022, which claimed only the priming half.
