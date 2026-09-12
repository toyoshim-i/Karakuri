# `karakuri-store`

Content-addressed storage (CAS), session journaling, and Set/Arrangement file persistence for the Karakuri system.

---

## 1. Role in Architecture

`karakuri-store` sits at **Layer 2 (Domain Extensions & Storage)** in the Karakuri workspace:

- **Content-Addressed Storage**: Hashes shader sources, assets, and artifacts via SHA-256 for immutable, deduplicated storage.
- **Append-Only Session Journals**: Serializes and deserializes continuous performance sessions using newline-delimited JSON (`.ndjson`) streams.
- **Set & Arrangement Persistence**: Reads and writes `.kset` files and UI layout configuration files (`.arrangement.json`).
- **Zero Engine/GPU Bindings**: Operates purely on disk files, memory maps (`memmap2`), and data structures without depending on graphics runtime libraries.

---

## 2. Key Invariants

1. **Deterministic Content Addressing**:
   - Stored artifacts are keyed by lowercase hex SHA-256 hashes (`sha256:...`). Identical content produces identical keys.
2. **Crash-Safe Append Streams**:
   - Session journals append records atomically with forward-compatible version headers (`schema.rs`). Unrecognized future record types pass through without aborting playback.
3. **Reproducible Replay**:
   - Session streams record deterministic parameter transitions, cuts, and tempo anchors required to recreate identical performance runs offscreen.

---

## 3. Testing & Verification

Run tests:

```sh
cargo test -p karakuri-store
```
