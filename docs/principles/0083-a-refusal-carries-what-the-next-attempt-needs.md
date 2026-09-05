# A refusal carries what the next attempt needs

A rejection is a message to the author of the next attempt, and that author is a program in a loop.
It carries the constraint and the numbers, never the correction, and every diagnostic in a file is
reported at once so the repair is one round trip.

**What it rules out.** *Over budget* as a message: a model cannot tell how much to remove. The form
that works names the figure and the dominant term — *6344 ops/element exceeds the 4096 ops/element
ceiling (dominated by `curl` in `element`: ~6144 of 6344 ops, 96%)*. A hint that corrects the syntax
rather than stating the reason: *loop bounds cannot reference a param, an ambient, or any other
expression — cost estimation needs them fixed at parse time* led the model to conclude that
integrating a streamline per sample is incompatible with the cost model itself and to switch method
entirely; a hint that had only fixed the `for` statement would have produced a procedure that
compiles and cannot be afforded. Naming the mistake instead of the fix, where reading `energy` gets
*the signal bus is not readable from IR — declare `param energy` and attach a `bind` record to it in
the Set file*. And severities: `IrResult<T> = Result<T, Vec<IrError>>` has no room for a second one,
an error means no artifact and there are no warnings, and adding severity while it is cheap adds a
concept with no user and invites diagnostics that neither stop a build nor get read.

**Where it holds.** [`error.rs`](../../crates/karakuri-ir/src/error.rs), where
`IrResult<T> = Result<T, Vec<IrError>>` is the type that makes one severity and one round trip the
same fact; [`check.rs`](../../crates/karakuri-ir/src/check.rs), which collects every diagnostic
rather than raising the first and carries the hint that names the fix; and
[`cost.rs`](../../crates/karakuri-ir/src/cost.rs), whose rejection prints the estimate, the ceiling
and the dominant term. Outside the IR it is
[`gate.rs`](../../crates/karakuri-operation/src/gate.rs)'s `refusal`, which names the operation, its
class and where the operator opens that class, so a closed instrument does not read as an incapable
one. [ir-spec.md](../ir-spec.md)'s stage list states the one-severity half. Decided in
[ADR-0012](../adr/0012-one-severity-and-a-rejection-carries-numbers.md),
[ADR-0086](../adr/0086-a-hint-says-why.md) and
[ADR-0235](../adr/0235-mcp-reaches-every-operation-and-what-could-stop-the-show-is-refused-until-the-operator-opens-it.md).
