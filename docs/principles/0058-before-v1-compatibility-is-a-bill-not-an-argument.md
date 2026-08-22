# Before v1, compatibility is a bill and not an argument

While the specification is still moving, a breaking change costs nothing in compatibility. **Until v1
is settled, pay the cost of changing toward whatever ideal you find.** Estimate the bill honestly when
presenting a fork — and do not let the bill decide the recommendation.

**What it rules out.** Treating rework as a reason. It kept a compatibility alias for `field(p)` on the
table when deleting it was plainly right, and it was about to make an address shift a reason not to
turn the built-in camera into an ordinary node — when the real risk there was something else entirely:
`slot_of` and `params` disagreeing, which shifts every renderer's address **silently**, because the
neighbouring node genuinely exists.

**It expires.** At v1 the bill starts being an argument, and this rule stops applying.

**Where it holds.** Decided in
[ADR-0117](../adr/0117-before-v1-pay-the-cost-of-changing-toward-the-ideal.md).
