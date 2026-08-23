An ADR is a **description of history**. The events it records are fixed; the description is a
document, and a document can be wrong. Those are different things, and every question about
editing one comes apart on that line.

**Revision is ruled out.** Editing a landed record so that it agrees with the present is
history revision: it destroys the only account of how the code came to be shaped this way,
and it does so silently, because the rewritten record reads exactly as plausibly as the
original. A record describing what a document said in August is *right* in August's terms and
must stay that way. If the argument itself would have to change, that is a new record. This is
also what buys permission to stop maintaining: nobody keeps a hundred and fifty records
current, and none of them has to be.

**Leaving a wrong description is ruled out too**, and this is the half that gets missed. A
record that misstated a fact when it was written, or whose pointer has gone bad, is not
history — it is a bad account of history, and "it is append-only" is not a reason to keep it.
Retiring a principle already requires the correction: re-point the ADRs that cited it.

**Annotation is the third case and it is useful.** Adding what a record later became —
`status`, `superseded_by`, `principles`, a pointer to what replaced it — adds information
*about* the past without changing what the record says happened. The front matter exists to be
written after the fact, which is the plainest evidence that a landed record was never meant to
be untouchable.

The test, when it is not obvious: **does the edit change what the record says happened, or
what a reader can find out about it?** The first is revision. The second is maintenance.

See [ADR-0151](../adr/0151-an-adr-is-a-description-of-history.md).
