# A `.kir` never names a node of a Set

A procedure declares the inputs it takes and gives each one **a name of its own** — `uses far :
Geometry`, `uses shape : Field`, `uses view : Camera`, `uses only : Source` — and the Set says what
fills it, as an `edge` record written `--edge morph.far=sphere_shell`. The name inside the
procedure is the *slot's*; the names in the record are *nodes'*. A node's address is
`(layer, index)` and its name is an alias for it.

**What this rules out.** A procedure that names its neighbour — `far = sphere_shell` written in the
`.kir`, which is shorter, needs no record, and is the reason this rule exists: a part that names
the parts around it is bound to one Set and stops being a library part. A corpus of procedures that
cannot be recombined is not a corpus.

**And it rules out "if there is exactly one, use it".** An unbound slot is refused, naming the slot
and listing what the Set holds. That rule is not a convenience with an edge case; it is the cap
itself — it is what held a Set to one field and one camera, and reinstating it under a new spelling
would cap the next fan-in the same way and be found out the same way, by a picture that changed
when nothing about the material did.

The price is that binding is a second statement: a Set with an unbound slot does not build, and
every surface that rebuilds one — `--watch`, a Set file, a hot swap — has to carry the names it was
spelled with rather than regenerate them.

See [ADR-0152](../adr/0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md), and
[ADR-0111](../adr/0111-a-name-lives-in-the-set-file-and-may-be-written-on-the-command-line.md) for
the identity half that came first.
