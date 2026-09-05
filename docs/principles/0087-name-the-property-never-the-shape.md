# Name the property, never the shape

What the code currently looks like is not the design. A branch, a field, a position, a channel's
present contents — each is one presentation of a requirement nobody wrote down, and a rule that fixes
the presentation leaves the requirement to be reinvented wherever the presentation does not fit, in a
form nothing advertises. So a fix is specified as **there is exactly one derivation of X**, and where
possible as which one is canonical. Whether that one place is a record or a read is decided by
whether the value must survive unrelated change: an identity is assigned once and recorded, because
every derivation of it moves when something unrelated moves; a pending state is derived every frame,
because a copy held in a surface goes stale with no assertion to write against it.

**What it rules out.** Deleting the branch you can see. Told to delete a `Sources::Startup` arm, the
arm went and the duplication relocated into launch seeding in a less visible form, harder to find
because what used to advertise it was gone; it took three rounds, and closed only by naming which
read is canonical. *Two lamps and a blink* as the rule, when the requirement is that a pending
control says three things — where it is, where it is going, and that it has not arrived — and the
fourth clause once written with them, *reads as unsettled in a single frame*, which ruled out every
presentation whose meaning is carried by change. A source's number derived from its position in a
merge, its place in a Set list, a content hash or a procedure's name: `proc drift_shell` is a *type*
name, so two uses of one lattice collapse, and `--watch` is this project's central loop, so a content
hash moves constantly — and a wrong `mask source == 1` comes out as a picture rather than as an
error. Reading a `Set.camera` field as evidence that the camera is per Set, when the algebra had said
`L4 : (Geometry, Camera) -> Texture` from the beginning; what makes that hard to catch is that it is
*answerable*, so two plausible answers look like a real fork. Assuming a channel is in range because
nothing has complained: alpha was dead data until `over` gave it a reader, and a coverage of 1.5 then
subtracted instead of hiding — 1029 negative colour channels, worst −54.34 — while deleting
`opacity` from the composite entirely left the whole suite green. And baking a capacity into an
artifact: the 65536 and the 262144 version of one procedure hash differently and carry different
previews, so the library, which is the asset, multiplies by every size anyone ever wanted. Declare
the range instead — `capacity [65536, 1048576] = 262144` — and record cost per element.

**Where it holds.** [ir-spec.md](../ir-spec.md) is the authority for three of them, and each is
written as the property rather than as the arrangement that currently satisfies it: *A `source` value
is assigned and recorded, never derived*, which tabulates the four derivations and the case each one
breaks on; *capacity / topology*, where an artifact declares a range and a default because the number
is turned per Set rather than being part of what the procedure is; and the layer algebra at the top,
whose `L4 : (Geometry, Camera) -> Texture` settles the camera's scope with no rule added.
[parse.rs](../../crates/karakuri-ir/src/parse.rs)'s `parse_capacity` is where the range stops being
optional — `capacity [<min>, <max>] = <default>` is the only spelling that parses, so a baked
capacity cannot enter an artifact.
[composite.wgsl](../../crates/karakuri-engine/src/shaders/composite.wgsl) computes `covered` once and
uses the same value for the hiding term and for the alpha it writes, which is where the channel that
acquired a reader got the range it had never had. And
[view.rs](../../crates/karakuri-console/src/view.rs) derives where a surface would be tempted to
store: `Strip::pending`, `Strip::gain_pending` and `Strip::opacity_pending` are the request compared
against the effective value every frame and nothing remembers that a move is pending, while
`View::phase` arrives as a value because this crate holds no clock — which is also what lets
`tests/parked.rs` and `tests/armed.rs` assert an animation at a phase they chose. Decided in
[ADR-0009](../adr/0009-capacity-is-a-dial-not-part-of-a-procedures-identity.md),
[ADR-0070](../adr/0070-a-channel-nobody-reads-is-a-free-variable.md),
[ADR-0096](../adr/0096-camera-is-an-edge-into-l4-and-an-existing-field-is-not-a-fact.md),
[ADR-0101](../adr/0101-a-sources-number-is-recorded-not-derived.md),
[ADR-0123](../adr/0123-a-fix-brief-names-the-property-not-the-shape.md),
[ADR-0188](../adr/0188-a-pending-transition-says-it-is-pending-and-no-surface-holds-the-rule.md) and
[ADR-0189](../adr/0189-motion-may-carry-the-meaning-and-a-stopped-animation-is-a-fault-to-report.md).
