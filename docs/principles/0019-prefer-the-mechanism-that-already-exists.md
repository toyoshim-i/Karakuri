# Prefer the mechanism that already exists

Before a feature gets a subsystem, check what it is made of. A palette of prompts that worked is
`origin.prompt` plus `tag` plus the previews and the genealogy already in the library — it is the
browser, filtered, and nothing is built. A variant pool is a family of pre-forked Sets sharing
most of their slots, so switching one is the Set switch that already exists.

**What it rules out.** The heavier design that keeps arriving first: for the palette it was a
versioned, content-addressed, first-class corpus object whose hashes go into `origin` so that two
artifacts from one prompt can be told apart. It was justified by reproducibility, which this
material does not want — generating repeatedly from one starting point and keeping the few you
like **is** the workflow, so machinery explaining why two outputs differ answers a question nobody
asked.

**Where it holds.** [roadmap.md](../roadmap.md), M4. Decided in
[ADR-0021](../adr/0021-a-palette-is-the-library-filtered-not-a-new-object.md).
