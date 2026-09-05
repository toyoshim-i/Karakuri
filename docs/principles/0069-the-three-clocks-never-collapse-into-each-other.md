# The three clocks never collapse into each other

Three timescales run at once, and what may happen on each is fixed:

| Clock | Period | What happens on it |
|---|---|---|
| Frame | 8–16 ms | GPU execution and parameter evaluation. Nothing else |
| Beat / bar | 0.5–4 s | Variant switching, parameter morphs, transitions — **selection** among options already prepared |
| Generation | seconds to minutes | Writing a procedure, checking it, compiling its shaders. A background worker |

**What this rules out is doing a slower clock's work on a faster one.** Allocating a buffer or
compiling a shader on the frame path, which is
[P-0091](0091-cost-is-known-before-it-is-paid.md) stated from the other
end. A beat-clock event that *computes* the material it is switching to rather than choosing among
material that is already built — the reason a hot swap arrives from a worker already compiled, and
the reason a transition is a function of `beats` and nothing else. And, when there is a generator,
any call to it on a path a frame waits for: an agent reacting to music is choosing from a pool it
prepared earlier and queueing generation for what it expects to need later.

**It is what makes the middle clock statable at all.** A transition schedules a move to happen at a
named instant on the beat grid and produces its value from `beats` alone, so the same records give
the same fade frame for frame on a machine running at a different rate, and a tempo correction
mid-fade is correct rather than a glitch. That property survives only while nothing on the beat
clock has to wait for something on the generation clock.

**Two of the three are built and the third is half built**, and this rule is most of what the third
one is: the build worker compiles off the render thread and hands over a finished Set, and nothing
generates a procedure yet. The rule is what keeps the generator off the frame path when one
arrives, which is why it is written before there is anything to constrain.

**Where it holds.** [`transition.rs`](../../crates/karakuri-engine/src/transition.rs),
[`swap.rs`](../../crates/karakuri-engine/src/swap.rs) and
[`deck.rs`](../../crates/karakuri-engine/src/deck.rs).

**No ADR.** This is part of the design skeleton rather than a decision anyone took against an
alternative: it was written down as the single most load-bearing idea in the design before there
was an engine to test it against, and everything built since has been arranged around it.
