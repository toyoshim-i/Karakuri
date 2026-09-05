# Take the mechanism that exists, and pay the bill now

Before a feature gets a subsystem, check what it is made of; the parts are usually already there
under other names, and the cheapest correct revision is a parameter write, then a fork with no
compile, then a rebuild. **Until v1 is settled, a compatibility cost is a bill rather than an
argument**: estimate it honestly when presenting a fork, and do not let the bill decide the
recommendation. **This rule expires** — at v1 the bill starts being an argument and that half stops
applying.

**What it rules out.** A versioned, content-addressed, first-class corpus object for saved prompts,
when a palette is `origin.prompt` plus `tag` plus the previews and genealogy the library already
holds — justified by reproducibility, which this material does not want, because generating
repeatedly from one starting point and keeping the few you like *is* the workflow. A subsystem for
variant pools, when the alternatives are nodes of one Set and selection picks among them — the
mechanism a Set already has. P-0019 reached for a different existing thing here, a family of
pre-forked Sets across deck slots, and
[ADR-0148](../adr/0148-a-variant-pool-is-a-set-and-the-deck-stays-a-mixer.md) rejected it: a pool is
a Set, and the deck stays a mixer. Treating every instruction as a generation request, which is the obvious
implementation, is orders of magnitude slower and throws away the parts that were already right —
told `speed` is `[0.0, 2.0]` and currently `0.15`, a model turns *slower* into `0.08` without
generating anything. Keeping `field(p)` as a compatibility alias when deleting it was plainly right,
which would have left the language carrying two spellings forever. Letting an address shift argue
against making the built-in camera an ordinary node, where the real danger was never the visible
shift but `slot_of` and `params` disagreeing, which moves every renderer's address **silently**
because the neighbouring node genuinely exists. Reaching for a script in another language for a
repository chore, when `cargo test` is the cheaper answer because it is already installed, and
vendoring a C++ dependency behind a build script, which charges the cost to everyone who builds the
repository including on platforms where the feature does not exist — Link is out of process for its
cmake and its C++ compiler alone, and **a second toolchain is a reason to put something outside and
never a licence to**. And waiting for a budget to fix waste that buys nothing: capacity buys
elements, a resident Set buys instant switching, sixteen bytes holding a four-byte `seed` buys
nothing at any size on any machine.

**Where it holds.** [Cargo.toml](../../Cargo.toml) is `members = ["crates/*"]` and nothing else:
fourteen Rust crates, no `build.rs` anywhere in the tree and no script in another language, so
`cargo` is the whole build and a second toolchain cannot enter unremarked. The out-of-process half
is [plugins.md](../plugins.md) and
[tempo_source.rs](../../crates/karakuri-environment/src/tempo_source.rs), the wire and the one helper
that exists. The revision ladder is enforced at the bottom rung:
[parse.rs](../../crates/karakuri-ir/src/parse.rs) refuses a `param` with no range — *the range is
mandatory* — which is what makes *a bit slower* answerable by a uniform write instead of by
regeneration, and [ir-spec.md](../ir-spec.md)'s `param` names that as the range's fourth job. The
palette half is specification rather than code and says so: `origin`, `parent` and `tag` are on the
metadata card with no producer yet, which is what a library has to hold before it can be filtered
into one. Decided in [ADR-0021](../adr/0021-a-palette-is-the-library-filtered-not-a-new-object.md)
and its successor
[ADR-0133](../adr/0133-what-was-fed-to-a-generator-is-a-field-not-an-object.md),
[ADR-0022](../adr/0022-revision-goes-param-then-range-then-regeneration.md),
[ADR-0074](../adr/0074-a-plugin-boundary-is-drawn-by-the-deterministic-path.md),
[ADR-0076](../adr/0076-commit-a-manifest-not-a-binary.md),
[ADR-0081](../adr/0081-ndjson-over-a-pipe-and-not-protobuf.md) and
[ADR-0117](../adr/0117-before-v1-pay-the-cost-of-changing-toward-the-ideal.md).
