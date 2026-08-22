# Mark what is preference, so it can be revisited without a redesign

Where a decision has a forced part and a chosen part, say which is which. The specification's section
on source identity marks two clauses **"preference rather than force"**, and when one of them turned
out to be wrong the mark is what made it a **one-clause revision instead of reopening the decision**.

**What it rules out.** Writing a settled section as though all of it were equally load-bearing. The
clause that failed — *a name is written in a Set file and not on the command line, since anyone who
needs to point at a source is already writing one* — was falsified by the code: a Set file cannot save
a chain or a second geometry, so **the person with two geometries is exactly the person who cannot
write a Set file.** Circular. Keeping the half that worked and dropping the half that did not was only
cheap because the halves were labelled.

**Where it holds.** [ir-spec.md](../ir-spec.md), "Multiple L1 sources, and `source`". Decided in
[ADR-0111](../adr/0111-a-name-lives-in-the-set-file-and-may-be-written-on-the-command-line.md).
