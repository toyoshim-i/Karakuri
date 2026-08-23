# Auditioning is a prerequisite, not a convenience

An operator can put any one slot on the output in place of the mix, **whatever that slot's
residency**. Choosing between candidates is the basic workflow of this instrument, and it cannot be
done blind.

**What this rules out is trading the control against its cost**, and the cost is real and written
down: an audition adds a draw that is not in the budget and falls inside the watchdog's judging
window, so auditioning a heavy slot can roll back an unrelated slot's build
([ADR-0072](../adr/0072-auditioning-adds-a-draw-and-never-a-step.md)). That is a bill to pay, not
an argument for removing the control or for hiding it behind a flag.

**And it rules out offering only the slots that are easy to offer.** An off-air slot is exactly the
one worth looking at — an Allocated one shows the still it stopped at, a Priming one shows what it
is warming into — so restricting the audition to Live slots would remove it from the only moment it
is needed. A still, a thumbnail or a scaled-down copy is not a substitute either: what is being
judged is moving material through the transfer curve it will be shown through.

The mechanism this rule needs is separate and already decided: an audition adds a draw and **never
a step**, or looking at a slot would move it —
[P-0041](0041-observing-must-not-advance-what-is-observed.md).

**Where it holds.** `Deck::set_preview` in
[`deck.rs`](../../crates/karakuri-engine/src/deck.rs), and the `v` key in `karakuri-cli`.

**No ADR.** Nothing was decided against an alternative here: the requirement was stated when the
instrument was first described and the control was built to it. ADR-0072 records how it behaves,
not whether it exists.
