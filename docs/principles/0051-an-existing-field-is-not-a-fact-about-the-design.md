# An existing field is not a fact about the design

A scope question — *per what, owned by what, how is it shared* — is checked first against the
declared type or the algebra. If the thing already appears there as an argument or an edge, **the
scope is settled**, and the question is a leak from the shape the current code happens to have.

**What it rules out.** Reasoning outward from an implementation field. `Set` held an `Orbit`, so
*is the camera per Set or per deck?* looked like a real fork — and the algebra had said
`L4 : (Geometry, Camera) -> Texture` from the beginning. Camera is an **edge into L4**, and sharing
is edge fan-out: one L3 read by two renderers is one viewpoint drawn twice, separate L3s composite
two viewpoints. Both fall out with no rule added.

**What makes it hard to catch is that it is answerable.** Two plausible answers look like a genuine
branch, and the wrong question reached a document as a request for someone else's judgement.

**Where it holds.** [ir-spec.md](../ir-spec.md), the layer algebra. Decided in
[ADR-0096](../adr/0096-camera-is-an-edge-into-l4-and-an-existing-field-is-not-a-fact.md).
