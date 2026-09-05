Every loop in a `.kir` has bounds that are signed integer literals. There is no `while` and
no recursion. Nested loops multiply into the estimate, and a range that does not ascend runs
zero times and is charged for zero times.

**What this buys is the whole safety story.** A procedure's cost is computed by the check
pass, before anything is built and before anything runs — which is what lets an expensive
mistake be *refused* rather than survived. Everything downstream depends on that number
existing: the frame budget a swapped-in Set is held to, the rollback that returns the previous
procedure when a candidate is over budget, and the promise that a model can write badly
without taking the show down.

**What it rules out** is any construct whose trip count is not knowable at parse time. A bound
that reads a param — so a procedure could adapt itself — is the one that keeps being reached
for, and it is why the checker's diagnostic states the constraint rather than the correction
(*"cost estimation needs them fixed at parse time"*, [P-0083](0083-a-refusal-carries-what-the-next-attempt-needs.md)).
`while` for an iterative solver and recursion for a tree walk are the other two. Each would
turn a price into a guess, and a governor that cannot price a candidate has nothing to do but
run it and watch — which is the frame this rule exists to protect.

The alternative that is sometimes proposed alongside them — estimate optimistically and rely
on the runtime rollback — moves a refusal that costs nothing into one that costs a dropped
frame in front of an audience, and only for procedures whose cost was never knowable in the
first place.

**Where it holds.** [ir-spec.md](../ir-spec.md), under the statement rules and "Cost
estimation"; `karakuri-ir`'s check pass and `cost::estimate`.
