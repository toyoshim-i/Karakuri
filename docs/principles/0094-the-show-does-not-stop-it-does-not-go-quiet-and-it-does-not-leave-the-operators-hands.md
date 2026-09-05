# The show does not stop, it does not go quiet, and it does not leave the operator's hands

Every mechanism that runs during a performance answers one question before it is built: what does this
do at its worst, on the frame it goes wrong, while the operator's attention is on the room? Three
answers are admissible — **it cannot happen**, refused or bounded where refusing is free; **it undoes
itself and puts the show back where it was**; or **it is loud, and the operator's hands still work**.
A mechanism that can give none of them does not run during one, and where more than one is available,
take the earliest: a refusal before the show costs nothing, a rollback costs a swap somebody asked
for, loudness costs the operator's attention at the moment they have least of it.

**The price is never the operator's.** Safety is bought with design, time, budget and loudness, never
with authority. The instrument refuses what it cannot do and never what it judges unwise. An
automatic recovery acts on what is off air, and where it must touch the live picture it may only put
it back where it was. A hand cancels whatever automatic thing was writing the control it lands on, on
every route in. A panic is admissible only where the alternative is a plausible wrong picture and the
condition cannot arise except from a bug; anything a night produces on its own — a costly Set, a NaN
in the mix, an unplugged interface, a dropped network — takes one of the three answers instead.

**What it rules out.** Leaving a bad Set on air until somebody notices, and swapping when the new
pipeline becomes ready, which is the moment easiest to detect and is inside a frame; pipelines are
double-buffered and exchanged between frames, and the parked Set is left unstepped so it returns
where it left off. A governor pulling a Live slot down mid-show — there is no recovery from it and no
way to explain it to a room — and steering that governor on the deck's frame interval, which cannot
separate one slot's cost from its neighbours', or on the host clock, which reacts to submission
overhead as though it were the Set's cost. Sanitising a NaN in the composite instead of skipping the
slot: `0.0 * NaN` is `NaN`, a probe found 47670 NaN channels in the mix from a slot whose fader was
at zero, and one NaN wiped out the surviving slot entirely — a fader pulled to zero is the last
resort for escaping broken material, so it has to work on exactly the material that is broken.
Buying frame budget by freezing the panel's continuous motion, 1.26 ms every 24.67 ms, which is the
cheapest thing the console has to say *this is live* and the only statement still working when
whatever would otherwise report the fault is itself what stopped. `dlopen`ing third-party code into
the render process, where a panic or a C++ exception crossing FFI is undefined behaviour that cannot
be caught, and a prompt box in the GUI, which makes API keys, model selection, streaming, retries and
rate limits this program's problem. A window that opens and cannot be touched, and a panel that
cannot meet a staleness it declared looking calm rather than saying it is over budget.

**And the mirror of all that: buying the safety back from the operator.** A confirmation before a
live action, a lock while something is pending, a "safe" mode that removes the control instead of
costing it. A machine that stops is a lost show; a machine that will not do what it is told is the
same lost show with a better excuse.

**Where it loses.** Auditioning adds a draw that is not in the budget, inside the window the swap
watchdog judges candidates in, so watching a heavy slot can roll back an unrelated slot's build. That
is unbudgeted risk on the live path, taken knowingly: **a rule protecting the performance may not be
used to remove what the performance is played with.**

**Where it holds.** The three answers are three places in the engine.
**Refuse**: `karakuri-ir`'s check pass and `cost::estimate` price a procedure before anything is
built, and [plugins.md](../plugins.md) keeps the network, the secrets and the foreign binary outside
the process, with a version handshake as the first call so an ABI mismatch refuses to load and names
both versions rather than rotting into a segfault mid-performance.
**Undo**: [swap.rs](../../crates/karakuri-engine/src/swap.rs) builds on a worker, polls with
`try_recv` and never `recv`, replaces `live` only at the top of `begin_frame`, discards
`WARMUP_FRAMES` before measuring `JUDGE_FRAMES` and takes a **median** — one hitch is not a reason to
throw away generated material and thirty frames that all miss is — and leaves the outgoing Set
unstepped, so a rollback resumes it exactly where it was parked.
[governor.rs](../../crates/karakuri-engine/src/governor.rs) budgets from the per-Set measurement the
worker took rather than from the deck's frame interval, moves slots between residencies **below**
Live, and warns.
**Be loud**: [deck.rs](../../crates/karakuri-engine/src/deck.rs)'s `Frame::render` checks its sizes
and panics at the call site, because `textureLoad` returns zero out of range by specification and a
mismatched `Present` would give a black frame every frame with nothing logged — and the module lists
what it does *not* enforce beside what it does, `mem::forget` on a frame guard included.
[composite.wgsl](../../crates/karakuri-engine/src/shaders/composite.wgsl) and
[mix.rs](../../crates/karakuri-engine/src/mix.rs) **skip** a slot that does not contribute rather
than multiplying it by zero. `view::BEAT_STALENESS` and `repaint::Change::Animating` in
[karakuri-console](../../crates/karakuri-console/) are the continuous motion, declared whether or not
anything is pending. **The operator's half** is enforced by a call rather than by a type:
`Deck::set_gain`, `Deck::set_opacity` and `Deck::set_mask_position` each call `Deck::cancel`, and
`Deck::set_mask_shape` deliberately does not, which
[tests/deck.rs](../../crates/karakuri-engine/tests/deck.rs) asserts in both directions;
[gate.rs](../../crates/karakuri-operation/src/gate.rs) holds the classes, the refusals and where each
one is opened, in the leaf every surface depends on rather than in one of them.
[manual.md](../manual.md) and `crates/karakuri/src/main.rs` are where an instrument says what it did.
**What no code enforces yet is the scheduler**: nothing arbitrates between declaring regions, so *a
scheduler may not stop the motion to make room* has nothing to bind, and the day something does
choose is the day this rule is what it is checked against. Decided in
[ADR-0040](../adr/0040-a-gain-of-zero-means-no-contribution-so-the-slot-is-skipped.md),
[ADR-0042](../adr/0042-a-silently-wrong-image-loses-to-a-loud-failure.md),
[ADR-0045](../adr/0045-the-cli-is-an-instrument-not-a-demo.md),
[ADR-0054](../adr/0054-the-governor-budgets-from-the-probe-and-never-touches-a-live-slot.md),
[ADR-0075](../adr/0075-the-plugin-abi-passes-a-handle-and-only-what-crosses-a-process.md),
[ADR-0083](../adr/0083-mcp-is-the-only-prompt-surface.md),
[ADR-0189](../adr/0189-motion-may-carry-the-meaning-and-a-stopped-animation-is-a-fault-to-report.md),
[ADR-0201](../adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md),
[ADR-0212](../adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md),
[ADR-0234](../adr/0234-carrying-a-show-through-is-a-principle-not-a-property-of-the-finished-instrument.md),
[ADR-0235](../adr/0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md),
[ADR-0242](../adr/0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)
and [ADR-0256](../adr/0256-a-swap-lands-on-a-frame-boundary-and-an-over-budget-set-rolls-back-without-being-asked.md).
