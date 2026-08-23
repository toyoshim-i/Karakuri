A source comment cites what is **in force** — a principle, an ADR, or a present-tense
document like `docs/ir-spec.md`. **It never cites a plan**, which means `docs/roadmap.md`
and anything under `docs/history/`.

**A milestone named without a filename is the same citation.** *"M2's budget governor"*,
*"deferred to M6"*, *"says M2 of the budget governor"* — these carry a schedule into the code
exactly as a path would, and they are harder to find because a search for the document does
not catch them. Where the thing exists, name the thing (`crate::governor`, not "M2's budget
governor"). Where it does not, say that it is not built rather than when it was going to be.

What this rules out is the citation that reads as authority and is a schedule:
*"`docs/roadmap.md`, M4, \"Naming what a Set holds\""*. A milestone describes work that was
intended, organised by when it was going to happen, and a reader who follows the pointer
arrives at that instead of at why the code is the way it is. When the milestone closes and
its section is rewritten, the comment still reads plausibly and nothing fails — the same
failure [P-0023](0023-a-document-that-describes-replaced-behaviour-is-worse-than-none.md)
names, with a compiler in the room that cannot see it.

Three moves, and one of them is usually right. Where the comment already states the claim,
**delete the pointer** — the sentence was carrying the argument and the pointer only
provenance. Where the reason is elsewhere and load-bearing, **cite an ADR or a principle**,
both of which are addressable and neither of which is a schedule. Where neither exists,
**the comment wanted an ADR that has not been written**, and writing it is the fix.

Counted on 2026-08-23, before this rule: 71 references to the roadmap across 26 source
files, against one reference to an ADR or a principle in the whole of `crates/`. See
[ADR-0149](../adr/0149-source-cites-what-is-in-force-not-a-plan.md).
