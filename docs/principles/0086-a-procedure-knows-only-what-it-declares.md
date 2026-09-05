# A procedure knows only what it declares

A `.kir` is authored once and instantiated by somebody else. How large its target is, how many
sources feed it, which nodes of which Set fill its inputs, what the room sounds like — none of that
is in the file and none of it is knowable from it. What reaches a procedure arrives through a
declaration: `uses` answered by an `edge`, `param` answered by a `bind`. Everything outside the
declaration is somebody else's to choose, and the mechanisms that keep it choosable are the price of
that rather than niceties — an extent stated as a fraction of the target's **height**, so an aspect
change moves the frame's edges rather than the sprite; a primitive below a pixel drawn at one pixel
and dimmed rather than dropped by the sample grid; a picture that letterboxes rather than stretches.

**What it rules out.** A unit in pixels, and a reference resolution in the specification, which only
invites a procedure to be authored against it and be wrong everywhere else. An ambient a procedure
could ask the render size with: the generated uniform carries `viewport` because the lowering needs
it to turn a fraction into a clip-space offset, and it is engine state, deliberately not an ambient.
`if source == 0` inside a renderer, which binds that L4 to one Set's arrangement, where one written
for a single source works unchanged with five. `far = sphere_shell` written into the `.kir`: shorter,
needs no record, and a part that names the parts around it is bound to one Set and stops being a
library part — a corpus of procedures that cannot be recombined is not a corpus. *If there is exactly
one, use it*, which is not a convenience with an edge case but **the cap itself**: it is what held a
Set to one field and one camera, and under a new spelling it would cap the next fan-in the same way
and be found out the same way, by a picture that changed when nothing about the material did. The
positional form it replaced, where `--set` order decided the second input, written nowhere, so the
same two geometries listed either way gave different pictures. And reading `energy` from inside a
shader, where the UI cannot show the coupling, an agent cannot adjust it and the record cannot carry
it, and where a curve, a range and a confidence blend have nowhere to live. `beats` is the stated
exception, with its reason: what follows a tempo is not a parameter value but the passage of time,
and a binding writes one number where a clock has to move everything the procedure does.

**Where it holds.** [ast.rs](../../crates/karakuri-ir/src/ast.rs): `UsesDecl` carries the slot's own
name, and `Ambient::ALL` is twelve values with no `viewport` and no bus name among them, so there is
nothing a procedure could ask the render size or the room with.
[check.rs](../../crates/karakuri-ir/src/check.rs) refuses a bus name with the sentence that names the
fix, and refuses `source` in a `camera` block, in a `field` block and in a procedure declaring a
`Geometry` slot. An unbound slot is refused where the Set is built rather than filled in from what is
lying around — `tests/field.rs::refused::an_unbound_field_slot_is_refused_rather_than_fatal` asserts both
halves, including a Set that *holds* exactly one field. [ir-spec.md](../ir-spec.md) is the authority
for the rest: *Ambient values and the signal rule*, *Multiple L1 sources*, and *`point_rate` is a
fraction of the target's height, and not a count of pixels*. Decided in
[ADR-0025](../adr/0025-sources-are-distinguished-downstream-and-a-renderer-never-branches-on-one.md),
[ADR-0111](../adr/0111-a-name-lives-in-the-set-file-and-may-be-written-on-the-command-line.md),
[ADR-0152](../adr/0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md),
[ADR-0245](../adr/0245-the-sub-pixel-compensation-is-paid-in-the-colour-because-alpha-is-coverage.md),
[ADR-0246](../adr/0246-the-render-size-belongs-to-the-output-and-the-sessions-canvas-is-only-its-default.md)
and [ADR-0251](../adr/0251-the-signal-bus-is-not-readable-from-ir-and-beats-is-the-one-exception.md).
