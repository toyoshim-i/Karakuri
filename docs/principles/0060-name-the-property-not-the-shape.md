# Name the property, not the shape

A fix is specified as **"there is exactly one derivation of X"**, and where possible as **which one is
canonical** — not as *delete that branch*.

**What it rules out.** Naming the shape you can see. Told to delete a `Sources::Startup` arm, the arm
went and **the duplication survived**: it moved into launch seeding, in a *less visible* form —
re-reading the same path seconds after the compile that had already read it. A second review found the
same class in its new home. It closed only when the brief said **carry the bytes from the read that
produced the compile**, naming which read is canonical.

**A shape is one instance of a property.** Delete the shape and the duplicate relocates to wherever
the thing is still recomputed — and it is **harder** to find afterwards, because the branch that used
to advertise it is gone.

**Where it holds.** Decided in
[ADR-0123](../adr/0123-a-fix-brief-names-the-property-not-the-shape.md), after the class had already
been closed four times in one milestone under the name *one fact derived in two places*.
