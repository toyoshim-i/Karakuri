# Structure and randomness are independently seeded

The hash builtins are salted from the seed stream; `seed` itself is not. So `hash1(seed)`
changes when a Set is re-seeded and `seed % 512u` does not — the dust can be re-rolled without
moving the lattice.

**What it rules out.** Mixing the stream value into `seed` itself and presenting one word. It
reads simpler and it destroys the control: re-seeding would move the structure too. Also rules
out deriving randomness from anything but an explicit seed stream.

**Note.** A hash is therefore **not a pure function of its argument** across Sets. Stated in the
builtins section, because it is surprising.

**Where it holds.** [ir-spec.md](../ir-spec.md). Decided in
[ADR-0003](../adr/0003-hash-builtins-are-salted-from-the-seed-stream.md), narrowed from per
layer to per source when a Set could hold two geometries.
