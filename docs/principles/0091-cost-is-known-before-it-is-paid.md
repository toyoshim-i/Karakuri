# Cost is known before it is paid

Nothing is discovered to be expensive while it is on air. **The language is bounded so a price can be
computed before anything is built**: every loop bound is a signed integer literal, there is no
`while` and no recursion, and nested loops multiply into the estimate — which is what lets an
expensive mistake be refused rather than survived, and a governor that cannot price a candidate has
nothing to do but run it and watch. **The frame path pays nothing it did not pay earlier**, and work
moved off the render thread is not off it until its side effects are. **Anything that must be live
names two numbers** — what its update costs, and how stale it may get in milliseconds. **A
classification made ahead is trusted at runtime, so it is conservative.**

**What it rules out.** Creating a pipeline lazily on first use, which is the ordinary way to write
this and moves an unbounded stall onto whichever frame happens to be first; and a 12 MB
whole-capacity upload left staged on the queue, flushed by the render thread's next submit, landing
the hitch on the very frame a swap was meant to be invisible to. A loop bound that reads a param so a
procedure can adapt itself, `while` for an iterative solver, recursion for a tree walk. Estimating
optimistically and relying on the runtime rollback, which turns a refusal that costs nothing into a
dropped frame in front of an audience, and only for procedures whose cost was never knowable. A
tolerance stated in frames, which loosens by the same factor when the rate drops and is most
permissive exactly when the machine is most loaded. A cost paid every frame and named nowhere.
Rate-limiting the panel to afford it: halving the rate clumps the cost rather than removing it, and a
periodic hitch is more visible than a constant one. Claiming closed form loosely — the classifier
emits no diagnostic, so it can only be wrong silently, and permissive-wrong puts a slot on air
unwarmed *while telling the governor it needed no warming*. A ceiling calibrated for one shape: the
512-op fragment ceiling stood in for capacity × sprite area × overdraw, but a fullscreen renderer is
exactly one canvas exactly once and a marcher is 32 to 64 iterations by its nature, so the proxy did
not constrain ray marching, **it prohibited it**. The same model, given its own shape, produces an
optimisation instead — naming `rot_y` as 18% of a 7022-op estimate. And a general option's fallback
beating a declaration: `--capacity`'s own default of 262144 won over a `.kir`'s declared 131072, so a
procedure written for 131072 ran at 262144 unless somebody who knew said otherwise.


**Where it holds.** The bound is in the grammar rather than in a convention:
[parse.rs](../../crates/karakuri-ir/src/parse.rs) parses a `for` bound as a signed integer literal
and nothing else, and its refusal states the constraint rather than the correction — *"loop bounds
cannot reference a param, an ambient, or any other expression — cost estimation needs them fixed at
parse time"*. [cost.rs](../../crates/karakuri-ir/src/cost.rs)'s `estimate` is what that buys: three
axes charged per live element × frame, per element's life and per covered pixel, never summed, each
against its own ceiling, and `fragment_ceiling` picks between the two calibrations by what the
procedure is. [ir-spec.md](../ir-spec.md)'s statement rules and *Cost estimation* are the
authority — no `while`, no recursion, nested loops multiplying, a range that does not ascend running
and charged for zero times — and the classification a runtime trusts is `is_closed_form` in
[check.rs](../../crates/karakuri-ir/src/check.rs), which is conservative in the one direction whose
wrong answer costs a warm-up rather than an unwarmed slot on air.
[lib.rs](../../crates/karakuri-engine/src/lib.rs) states the frame-path half for the engine crate,
[swap.rs](../../crates/karakuri-engine/src/swap.rs) is where the double buffering and the
graveyard's `try_lock` live, and `deck.rs`'s frame guard owns the encoder. The two numbers are
[budget.rs](../../crates/karakuri-console/src/budget.rs) — `Declared { region, cost, staleness }`,
`PANEL_PASS` at 1.26 ms, and a `BUDGET` that is still the whole frame's because no panel share has
ever been written down — with `view::BEAT_STALENESS` at 24.67 ms and `view::ROLL_STALENESS` at
33.33 ms, both in milliseconds and neither in frames;
[tests/schedulable.rs](../../crates/karakuri-console/tests/schedulable.rs) asserts both conditions
rather than a stage discovering them, and `crates/karakuri`'s `WRITTEN_ALLOCS` is what keeps the
measured half honest. **A third field sits beside those two and is not one of them**:
`Declared::moves_in` is *when this region's picture next changes*, a function of the frame rather
than a constant of the presentation, so that a region pending and at rest stops asking for a rate it
is not using — the two numbers this rule names are still the two it names, and are still the two the
arithmetic is taken over
([ADR-0283](../adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)). `capacity_for` and `capacities_for` in
[karakuri-cli](../../crates/karakuri-cli/src/main.rs) are where a flag's default stops outranking a
declaration. Decided in
[ADR-0013](../adr/0013-cost-has-three-axes-that-must-not-be-added.md),
[ADR-0028](../adr/0028-closed-form-and-accumulating-is-a-static-classification.md),
[ADR-0033](../adr/0033-freeing-on-the-render-thread-is-the-same-invariant-as-allocating.md),
[ADR-0058](../adr/0058-closed-form-is-worth-more-for-scrubbing-than-for-priming.md),
[ADR-0090](../adr/0090-a-ceiling-calibrated-for-one-shape-rejects-the-next.md),
[ADR-0164](../adr/0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md),
[ADR-0210](../adr/0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md),
[ADR-0252](../adr/0252-the-language-is-bounded-so-a-price-can-be-computed-before-anything-is-built.md),
[ADR-0253](../adr/0253-a-flags-default-does-not-outrank-a-declaration-and-each-source-is-asked-separately.md)
and [ADR-0254](../adr/0254-a-written-figure-is-re-measured-rather-than-predicted-from-the-change-somebody-remembered.md).
