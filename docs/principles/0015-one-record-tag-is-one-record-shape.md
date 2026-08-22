# One record tag is one record shape

A `t` value means the same shape in every file. A decoder dispatches on `t` alone and needs to
know nothing about which stream it is reading, so a record can be quoted or moved between them.

**What it rules out.** Reusing a tag for a differently shaped record in a different file — which
is what `param` was doing across the metadata and the Set file, introduced and shipped within
hours by someone holding the whole format in mind. It follows that an unknown record is preserved
**verbatim** on a read-write round trip: re-serialising a payload-free `Unknown` would fabricate
`{"t":"unknown"}` and destroy the line, which is worse than dropping it because it looks faithful.

**Where it holds.** [ir-spec.md](../ir-spec.md); `ndjson.rs` in
[karakuri-store](../../crates/karakuri-store). Decided in
[ADR-0010](../adr/0010-one-t-value-is-one-record-shape.md).
