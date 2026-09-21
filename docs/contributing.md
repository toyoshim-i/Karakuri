# Karakuri Contributing & Development Guide

This document outlines the engineering principles, workflow guidelines, build and test commands, and verification practices for **Karakuri**. Both human contributors and AI coding agents MUST adhere to these rules when submitting changes.

---

## 1. Project Invariants & Core Rules

### Design Correctness is the Foremost Priority
In Karakuri, **architectural and design elegance, modularity, and correctness take precedence above all else**.
Code must never be contorted to accommodate hasty hacks or accumulate technical debt just to "make something work."
Every change should aim to leave the codebase cleaner, more cohesive, and easier to reason about than before.
When a choice arises between preserving an awkward historical convention and adopting a demonstrably cleaner design, **design correctness wins**.

### Separate Specifications from Arguments (No Arguments in Code or Manuals)
To maintain clarity across documentation and prevent conversational monologue from leaking into production artifacts:
- **ADRs are Arguments (Rationale)**: An Architectural Decision Record ([`docs/adr/`](adr/)) records *why* a decision was made, historical context, rejected alternatives, and design trade-offs.
- **Manuals and Code Comments are Specifications**: Manuals ([`docs/manual.md`](manual.md), [`docs/manual/`](manual/)), UI tooltips, and source comments (`//!`, `///`) record *what* the system does, observable behavior, invariants, and how to use the interface.
- **Never write arguments, historical narratives, or philosophical justifications in manuals, tooltips, or code comments.** Keep manuals focused strictly on user-facing behavior, and code comments on technical contracts, pre/post-conditions, and invariants. If you need to explain why an alternative was rejected, write an ADR.

### All Committed Content Must Be in English
All files committed to the repository—including Rust source code, test suites, doc comments (`//!`, `///`, `//`), specifications, architecture documents, ADRs, examples, and git commit messages—**MUST be written strictly in English**.
- **No verbatim chat logs in ADRs or comments**: Do not dump non-English chat excerpts, raw conversational quotes, or assistant monologue into committed documents or code comments.
- **Technical distillation**: When capturing maintainer decisions, feedback, or rationale originally expressed in other languages (e.g. Japanese), translate and synthesize the requirements into clean, objective, technical English before committing.

### Rapidly Understanding the Codebase Architecture (3-Tier Hierarchy)
Before writing code or proposing changes, contributors and AI agents MUST understand the system topology and component boundaries. The codebase is documented in a three-tier hierarchy designed for quick onboarding:

1. **System-wide Architecture ([`docs/architecture.md`](architecture.md))**:
   The top-level map covering repository-wide topologies, the 16-crate division of labor, the frame pipeline, and the render thread model. Start here to understand how the entire instrument fits together.
2. **Subsystem Architecture ([`docs/architecture/`](architecture/))**:
   Domain-specific deep dives into individual subsystems:
   - [`console.md`](architecture/console.md): Egui UI layout, componentized widgets, modular bays, and Control descriptor registry
   - [`engine.md`](architecture/engine.md): 8-stage `.kir` compilation pipeline, Deck/Set execution graphs, residency lifecycle, and 3-clock model
   - [`runtime.md`](architecture/runtime.md): Desktop GUI coordinator (`karakuri`), headless runner (`karakuri-cli`), environmental services, and artifact store
   - [`operations.md`](architecture/operations.md): Single operation vocabulary (`karakuri-operation`), immutable session journals, sequencer, and MCP server
   Read the subsystem document that corresponds to the area you are modifying.
3. **Crate-Level API & Invariants (`crates/<crate>/README.md`)**:
   Every single one of the 16 crates maintains its own `README.md` documenting its public types, internal module hierarchy, dependencies, and testing rules. Consult the crate `README.md` before editing files in that crate.

### ADRs are Historical Records, Not Inviolable Laws
Past decisions are recorded in [docs/adr/](adr/). However, **do not treat ADRs as immutable dogma or religious law**:
- An ADR is a *description of history* ([ADR-0151](adr/0151-an-adr-is-a-description-of-history.md)) capturing why a particular alternative was chosen under the constraints, knowledge, and state of the repository *at that specific moment in time*.
- As the system evolves, past decisions may become suboptimal, restrictive, or obsolete.
- **Always critically assess the validity of existing ADRs against current design ideals.** If an old ADR forces unnatural contortions, preserves technical debt, or contradicts clean architecture, challenge it.
- When a past decision is superseded by a cleaner design, record a superseding ADR (e.g. `ADR-0344` superseding `ADR-0049`) and modernize the code. **Design correctness trumps historical inertia.**

### Project Principles
Standing invariants are maintained in [docs/principles/](principles/) — **one file per rule**:
- Each rule is stated once and cannot drift across multiple documents.
- The filenames in `docs/principles/` form the index of active rules.
- Key principles to review before modifying the engine:
  - [Cost is known before it is paid](principles/0091-cost-is-known-before-it-is-paid.md)
  - [The same inputs produce the same frame](principles/0092-the-same-inputs-produce-the-same-frame.md)
  - [Pipeline is linear HDR with sRGB encoded once](principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)
- When a principle becomes obsolete, retire its file and record the retirement in [docs/adr/INDEX.md](adr/INDEX.md) rather than silently editing it into a different rule ([ADR-0059](adr/0059-a-records-pointer-into-the-principles-registry-is-metadata.md)).

### Engineering Style & Constraints
- **Vertical slices over horizontal speculation**: Deliver working, end-to-end functionality (e.g., getting a primitive on screen reliably) rather than building ungrounded abstraction layers.
- **Keep code continuously buildable**: Never commit changes that break the build or fail automated checks.
- **Validate abstractions**: Do not introduce a shared abstraction without at least two distinct concrete call sites.
- **File length limits**: Source files exceeding 1,800 lines are strictly prohibited and rejected by git pre-commit hooks. When a Rust source file exceeds 1,000 lines, git hooks issue a reminder ([ADR-0345](adr/0345-a-file-that-crosses-1000-lines-gets-a-nudge-not-a-gate.md)). Modularize large files into focused submodules (see [docs/refactoring.md](refactoring.md)).
- **Performance Benchmarking Standards**:
  - GPU timestamp queries can be unreliable or unsupported depending on OS and driver backends ([ADR-0169](adr/0169-the-timestamp-verdict-is-the-backends-not-the-machines.md)). Probes calibrate against known workloads and fall back to host-side timers where needed.
  - Performance comparisons must use the standardized reference workload: [`examples/drift_cloud.kset`](../examples/drift_cloud.kset) (262,144 elements, rendered at 1280x720; [ADR-0270](adr/0270-the-reference-workload-is-a-named-set-rather-than-whatever-the-default-pair-is.md)).
  - Explicitly document measurement conditions (canvas size, element count, host vs. GPU timing) alongside any reported figures.

### Git Hygiene & Workflow
- **Stage files explicitly**: Always specify paths explicitly with `git add <file>`. **Never use wildcard `git add -A`, `git add .`, or `git commit -a`**, which can inadvertently stage temporary artifacts or unreviewed edits.
- **Verify status before staging**: Run `git status` and `git diff` to inspect changes thoroughly before staging and committing.
- **Preserve unrelated working changes**: If the working tree contains uncommitted files from another session or task, do not clean, revert, or commit them. Stage only the files relevant to your task.
- **Atomic, focused commits**: Group related changes into single, coherent commits where documentation and implementation updates land together.
- **Clear, descriptive commit messages**: Explain what changed and why in the commit message. All commit messages MUST be in English. For work co-authored with AI assistants, append the standard trailer:
  `Co-Authored-By: <Agent Name> <<agent-email>>`
- **English-only committed artifacts**: Ensure all committed files, comments, and documentation are strictly in English.
- **Do not push without review**: Commits should be staged and committed locally; pushing to remote repositories is handled by the maintainer.

---

## 2. Environment Setup & Development Tools

### Required Prerequisites
- **Rust**: Stable Rust (`1.84` or higher) with `cargo`.
- **GPU Driver**: Vulkan, Metal, or DirectX 12 compatible driver with WebGPU support.

### Git Hooks
Enable repository hooks located in `.githooks/`:

```sh
git config core.hooksPath .githooks
```

- **`pre-commit`**: Runs `cargo fmt --check` against staged Rust files to enforce workspace formatting standards, strictly rejects any file exceeding 1,800 lines, enforces workspace-wide Clippy cleanliness (`cargo clippy --workspace --all-targets -- -D warnings`), and warns if a modified file exceeds 1,000 lines for the first time.
- **`pre-push`**: Runs full workspace formatting, lints, and test suites on tag pushes.

---

## 3. Build, Lint, and Test Commands

### Running Workspace Tests
The full test suite can be run across the workspace:

```sh
cargo test --workspace
```

### Testing Specific Crates
To run tests for individual crates during targeted development:

```sh
cargo test -p karakuri-ir          # IR parser, type-checker, and cost estimator
cargo test -p karakuri-codegen     # WGSL shader generation and naga validation
cargo test -p karakuri-engine      # Render graph and Set execution (requires GPU)
cargo test -p karakuri-audio       # Audio FFT analysis and beat tracking
cargo test -p karakuri-signal      # Signal bus, oscillators, and noise
cargo test -p karakuri-store       # Content-addressed store and ndjson journals
cargo test -p karakuri-midi        # MIDI message parsing and control mapping
cargo test -p karakuri-operation   # Operation vocabulary verification
cargo test -p karakuri-operation-record # Operation journal serialization
cargo test -p karakuri-layout      # Layout arithmetic and panel geometries
cargo test -p karakuri-environment # Setfiles, file watching, and presets
cargo test -p karakuri-console     # VJ console UI model and widgets (pure CPU)
cargo test -p karakuri            # Desktop GUI application integration (requires GPU)
cargo test -p karakuri-cli         # Headless CLI runner and replay tests (requires GPU)
```

### Running CPU-Only Tests (Skipping GPU)
Tests requiring physical GPU access are organized under modules named `gpu`. To execute all unit tests quickly without requiring GPU initialization:

```sh
cargo test --workspace -- --skip gpu::
cargo test -p karakuri-engine -- --skip gpu::
```

### Test Quality & Verification Practices
- **Observe failure first (TDD)**: Verify that a new test actually fails against broken or missing code before confirming it passes against the solution.
- **Negative controls**: Checker and validator tests must pair rejection assertions with corresponding acceptance cases to ensure the validator is not simply rejecting everything.
- **Compile-fail doctests**: Pair `compile_fail` doctests with compiling equivalents differing strictly by the invariant under test.
- **Independent fixtures**: Never use live, editable application presets as static test fixtures. Store static test fixtures under dedicated `fixtures/` directories where product mutations cannot alter them.

### Running Workspace Linter (Clippy)
```sh
cargo clippy --workspace --all-targets -- -D warnings
```

### Checking Code Formatting
```sh
cargo fmt --all -- --check
```

---

## 4. Recording Decisions & Preventing Drift

### When to Write an ADR
- Write an Architectural Decision Record ([`docs/adr/`](adr/)) whenever a design choice rejects a viable, non-trivial alternative that someone might reasonably reconsider in the future.
- Focus the ADR on the **context, problem statement, rejected alternatives, and trade-offs**.
- Number ADRs sequentially after the highest existing number, register the entry in [`docs/adr/INDEX.md`](adr/INDEX.md), and follow the format established in [`ADR-0000`](adr/0000-record-decisions-here-and-standing-rules-in-principles.md).

### When to Add a Principle
- Add a file to [`docs/principles/`](principles/) only when an invariant applies broadly across multiple subsystems and resolves questions without needing to mention specific components directly ([ADR-0249](adr/0249-a-principle-is-what-decides-a-question-it-does-not-mention.md)).
- If a rule addresses only a localized decision, record it as an ADR rather than a principle.

### Preventing Documentation Drift
To prevent discrepancies between code and documentation:
1. **Generated Sources**: Wherever possible, generate documentation and schemas directly from code constants (e.g. `Builtin::ALL` in `karakuri-ir`).
2. **Automated Invariant Tests**: Verify architectural rules through automated tests (e.g. `no_clock_access.rs` ensuring the signal bus does not read host clocks).
3. **Type-Enforced Invariants**: Use Rust's type system, affine ownership, and RAII guards to make illegal states unrepresentable (e.g. `FrameGuard` preventing mid-frame encoder re-acquisition).
4. **Cite Active State, Never Schedules**: Source code comments must cite active principles or ADRs, never milestones, plans, or roadmap schedules ([ADR-0149](adr/0149-source-cites-what-is-in-force-not-a-plan.md)).

---

## 5. Adding or Updating an Operation

In Karakuri, every user or automated action is modeled as an `Operation` ([`karakuri-operation`](crates/karakuri-operation)). When adding, modifying, or retiring an operation, update all associated locations in the following sequence:

1. **`docs/manual/operations.html`**: Add, update, or remove the operation row, description, and status badges.
2. **`docs/manual/console.html`**: Update corresponding control tooltips (`data-tip`) and UI documentation.
3. **`crates/karakuri-operation/src/lib.rs`**: Update the `Operation` enum variant and ensure its title string matches the manual heading exactly.
4. **`crates/karakuri-operation/src/gate.rs`**: Assign the operation to its appropriate safety permission class.
5. **`crates/karakuri-operation-record/src/lib.rs`**: Define how the operation serializes into session journals (`Records`, `Silent`, or `Owed`).
6. **`crates/karakuri-store/src/record.rs`**: Update persistent journal record formats if new disk serialization is introduced.
7. **Input Surface Bindings**: Connect the operation to UI handlers in `karakuri-console`, keyboard bindings in `karakuri`, CLI flags in `karakuri-cli`, or MIDI maps in `karakuri-midi`.
8. **Automated Verification**: Run integration tests verifying operation consistency:
   ```sh
   cargo test -p karakuri-operation --test the_manual_and_the_vocabulary_agree
   cargo test -p karakuri-console --test panel_column
   ```
9. **Record ADR Consequences**: Record any operational changes and rejected routing alternatives in a dedicated ADR.

---

## 6. Carrying Out Non-Operation Architectural Decisions

For architectural modifications that do not involve operations (e.g., DSL syntax changes, storage refactoring, widget componentization):
1. **Audit Dependencies**: Survey all affected files across workspace crates before modifying code.
2. **Specification First**: Update the formal specification first ([`docs/ir-spec.md`](ir-spec.md), [`docs/architecture/`](architecture/), or relevant crate `README.md`).
3. **Implement Changes Incrementally**: Break work into cohesive, single-file or single-crate steps.
4. **Run Verification**: Ensure all workspace unit, integration, and formatting checks pass.
5. **Document Outcomes**: Summarize changes factually in commit messages and relevant architectural documents.

---

## 7. Verification Checklist for Code Changes

Before marking a task or pull request as complete, verify that:

- [ ] **Distinguish specification from argument in all added text**: Every added documentation block (whether in `.rs` docstrings/comments, HTML manuals, or UI tooltips) MUST be a specification (what it does / how to use it). Any design arguments, historical context, or rejected alternatives belong strictly in an ADR, not in code or manuals.
- [ ] **All committed content is strictly in English**: All newly added or modified source code, tests, doc comments, specifications, ADRs, documentation, and commit messages are in English without raw non-English chat logs or quotes.
- [ ] **A decision with a losing alternative has an ADR**: Recorded in `docs/adr/` and registered in `docs/adr/INDEX.md`.
- [ ] **Operation changes updated across all surfaces**: Checked against Section 5 steps and validated with vocabulary tests.
- [ ] **All workspace tests pass**: `cargo test --workspace` completes successfully with zero failures.
- [ ] **Formatting and linter clean**: `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass without warnings.
- [ ] **Naga shader validation passes**: Modifications to `karakuri-codegen` verified against `naga_test.rs`.
- [ ] **Documentation and comments updated**: Docstrings, crate READMEs, and architecture docs reflect current reality without obsolete historical claims.
- [ ] **Explicit error handling**: Functions return structured domain errors (`IrError`, `SetError`, `StoreError`) rather than unhandled panics or silent fallbacks.
- [ ] **Relative links in documentation**: Markdown references use relative links (e.g. `architecture.md`, `../crates/karakuri-ir`).

---

## 8. Related Architecture & Specification Reference

### Tier 1: System Architecture
- [architecture.md](architecture.md): Source code structure, 16-crate dependency map, frame pipeline, and threading model
- [architecture/README.md](architecture/README.md): Architecture guide hub, 3-tier model overview, and crate topology diagrams

### Tier 2: Subsystem Architecture
- [architecture/console.md](architecture/console.md): Console UI layout, componentized widgets, modular bays, and Control descriptor registry
- [architecture/engine.md](architecture/engine.md): 8-stage `.kir` compilation pipeline, Deck/Set execution graphs, residency lifecycle, and 3-clock model
- [architecture/runtime.md](architecture/runtime.md): Desktop GUI coordinator (`karakuri`), headless runner (`karakuri-cli`), environmental services, and artifact store
- [architecture/operations.md](architecture/operations.md): Single operation vocabulary (`karakuri-operation`), immutable session journals, sequencer, and MCP server

### Tier 3: Invariants, Specifications & Crate Guides
- [crates/*/README.md](../crates/): Per-crate component guides, module boundaries, public APIs, and testing constraints for all 16 workspace crates
- [ir-spec.md](ir-spec.md): `.kir` DSL specification, type system, and language invariants
- [principles/](principles/): The rules in force, one per file — current non-negotiable invariants
- [adr/](adr/): Architectural decision records — historical context of past choices (descriptions of history, not immutable dogma)

### User Guides & Project Milestones
- [manual.md](manual.md): CLI arguments and VJ keyboard controls reference
- [manual/](manual/): The console's manual, published at <https://toyoshim-i.github.io/Karakuri/manual/>
- [plugins.md](plugins.md): Out-of-process helper plugin specification
- [roadmap.md](roadmap.md): Milestone progress, delivery status, and technical backlog
- [history/](history/): Closed milestones archive
