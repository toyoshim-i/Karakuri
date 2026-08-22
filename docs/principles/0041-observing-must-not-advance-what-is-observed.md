# Observing must not advance what is observed

Auditioning a slot adds a draw and **never a step**. `t` moves only through `Set::prepare`, which is
what lets a slot be taken down and brought back where it stopped, and looking at one must not disturb
that.

**What it rules out.** A preview that steps the slot it previews. It fails in the least visible way
there is: you watch a moving picture, decide you like it, put it on air, and it is **not where you were
looking**. It also rules out a second render pass for the purpose — running the ordinary mix with the
target at unity and the rest skipped puts that slot's texels straight onto the target, leaving the tone
mapper, the present pass and the readback untouched.

**Faders are ignored while auditioning**, on purpose: what is being judged is the level the material
*arrives* at, which is the input to setting a fader rather than the result of having set one. So the
audited slot's meter runs. That is the difference between looking and auditioning.

**Where it holds.** [deck.rs](../../crates/karakuri-engine/src/deck.rs). Decided in
[ADR-0072](../adr/0072-auditioning-adds-a-draw-and-never-a-step.md).
