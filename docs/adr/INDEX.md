# Architecture Decision Records

How this directory and [../principles/](../principles/) are run is
[ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md). Read that first.

**An ADR is a description of history**, which decides what may be edited: the past is not
revised, a description that was wrong is corrected, and annotating a record with what it later
became is welcome. The test is whether an edit changes what the record says happened or what a
reader can find out about it —
[P-0066](../principles/0066-an-adr-is-a-description-of-history-corrected-but-never-revised.md).

A retired number is never reused; when one is superseded it stays in the table with a pointer
to what replaced it. This index is maintained by hand — see ADR-0000 for when that stops being
enough.

`principles/` has no index. `ls docs/principles/` is the index, because each filename is the
rule it states.

**The reconstruction is complete**, including the work of 2026-08-22 itself, which was still being
committed in another session while this was being written. Every day of the session history from
2026-07-25 onward has been read and its decisions recovered. Numbering is chronological, so a later date takes a later
number; new records continue from the end.

## Records

| | Decision | Date | Status |
| --- | --- | --- | --- |
| [ADR-0000](0000-record-decisions-here-and-standing-rules-in-principles.md) | Record decisions here and standing rules in principles/ | 2026-08-22 | accepted |
| [ADR-0001](0001-identity-is-seed-an-ordinal-not-a-slot-index.md) | Identity is `seed`, an ordinal, and `id` is dropped | 2026-07-25 | accepted |
| [ADR-0002](0002-compaction-preserves-order-and-determinism-is-bit-exact.md) | Compaction preserves order, and determinism means bit-exact | 2026-07-25 | accepted |
| [ADR-0003](0003-hash-builtins-are-salted-from-the-seed-stream.md) | Hash builtins are salted from the layer's seed stream | 2026-07-25 | **superseded by ADR-0024** |
| [ADR-0004](0004-var-is-added-so-a-loop-can-carry-a-value.md) | `var` is added so a loop can carry a value | 2026-07-25 | accepted |
| [ADR-0005](0005-spawn-is-an-accumulator-and-the-engine-owns-the-birth-fraction.md) | Spawn is an accumulator, and the engine owns the birth fraction | 2026-07-25 | accepted |
| [ADR-0006](0006-the-step-count-is-a-record-not-a-measurement.md) | The step count is a record, not a measurement | 2026-07-25 | accepted |
| [ADR-0007](0007-a-set-file-is-a-projection-and-a-session-stream-is-the-timeline.md) | A Set file is a projection; a session stream is the timeline | 2026-07-25 | accepted |
| [ADR-0008](0008-blend-is-declared-now-so-the-second-mode-is-an-addition.md) | `blend` is declared now, so the second mode is an addition | 2026-07-25 | accepted |
| [ADR-0009](0009-capacity-is-a-dial-not-part-of-a-procedures-identity.md) | `capacity` is a dial, not part of a procedure's identity | 2026-07-25 | accepted |
| [ADR-0010](0010-one-t-value-is-one-record-shape.md) | One `t` value is one record shape, across every file | 2026-07-25 | accepted |
| [ADR-0011](0011-a-noise-binding-carries-a-kind-and-a-rate-in-beats.md) | A noise binding carries a kind and a rate in beats | 2026-07-25 | accepted |
| [ADR-0012](0012-one-severity-and-a-rejection-carries-numbers.md) | One severity, and a rejection carries the numbers | 2026-07-25 | accepted |
| [ADR-0013](0013-cost-has-three-axes-that-must-not-be-added.md) | Cost has three axes, and they must not be added | 2026-07-25 | accepted |
| [ADR-0014](0014-generated-code-cannot-be-captured-by-a-name-a-procedure-can-spell.md) | Generated code cannot be captured by a name a procedure can spell | 2026-07-25 | accepted |
| [ADR-0015](0015-a-measurement-carries-how-it-was-taken.md) | A measurement carries how it was taken | 2026-07-25 | accepted |
| [ADR-0016](0016-agents-leave-work-in-the-tree-and-the-reviewer-commits.md) | Agents leave work in the tree; the reviewer commits | 2026-07-25 | accepted |
| [ADR-0017](0017-an-invariant-that-can-be-tested-is-a-test.md) | An invariant that can be tested is a test, not a sentence | 2026-07-25 | accepted |
| [ADR-0018](0018-three-documents-three-jobs.md) | Three documents, three jobs | 2026-07-26 | accepted |
| [ADR-0019](0019-exposure-is-three-things-and-none-stands-in-for-another.md) | Exposure is three things, and none of them stands in for another | 2026-07-26 | accepted |
| [ADR-0020](0020-a-corpus-expresses-taste-and-never-a-missing-feature.md) | A corpus expresses taste, and never a missing feature | 2026-07-26 | accepted |
| [ADR-0021](0021-a-palette-is-the-library-filtered-not-a-new-object.md) | A palette is the library filtered, not a new object | 2026-07-26 | **superseded by ADR-0133** |
| [ADR-0022](0022-revision-goes-param-then-range-then-regeneration.md) | Revision goes param, then range, then regeneration | 2026-07-26 | accepted |
| [ADR-0023](0023-regeneration-is-destructive-in-a-slot-and-safe-in-the-library.md) | Regeneration is destructive in a slot and non-destructive in the library | 2026-07-26 | accepted |
| [ADR-0024](0024-each-source-counts-from-zero-and-carries-a-source-attribute.md) | Each source counts from zero and carries a `source` attribute | 2026-07-26 | accepted |
| [ADR-0025](0025-sources-are-distinguished-downstream-and-a-renderer-never-branches-on-one.md) | Sources are distinguished downstream, and a renderer never branches on one | 2026-07-26 | accepted |
| [ADR-0026](0026-the-deck-is-l5s-surface-and-its-size-is-two-budgets.md) | The deck is L5's surface, and its size is two budgets | 2026-07-26 | accepted |
| [ADR-0027](0027-a-set-value-is-immutable-and-its-compiled-instance-is-not.md) | A Set value is immutable; its compiled instance is not | 2026-07-26 | accepted |
| [ADR-0028](0028-closed-form-and-accumulating-is-a-static-classification.md) | Closed-form and accumulating is a static classification | 2026-07-26 | accepted |
| [ADR-0029](0029-amplification-is-a-second-kind-of-l2-not-a-new-layer.md) | Amplification is a second kind of L2, not a new layer | 2026-07-26 | accepted |
| [ADR-0030](0030-simulation-time-comes-from-an-integer-step-count.md) | Simulation time comes from an integer step count, and advances per substep | 2026-07-27 | accepted |
| [ADR-0031](0031-a-document-describing-replaced-behaviour-is-worse-than-none.md) | A document describing replaced behaviour is worse than none | 2026-07-27 | accepted |
| [ADR-0032](0032-nothing-checks-clean-and-comes-up-short-at-runtime.md) | Nothing checks clean and comes up short at runtime | 2026-07-30 | accepted |
| [ADR-0033](0033-freeing-on-the-render-thread-is-the-same-invariant-as-allocating.md) | Freeing on the render thread is the same invariant as allocating | 2026-07-27 | accepted |
| [ADR-0034](0034-the-frame-guard-owns-the-encoder.md) | The frame guard owns the encoder | 2026-07-30 | accepted |
| [ADR-0035](0035-claude-is-removed-from-history-while-there-is-no-remote.md) | `.claude/` is removed from history while there is no remote | 2026-07-30 | accepted |
| [ADR-0036](0036-the-first-m2-slice-is-two-sets-mixed-through-a-tone-mapper.md) | The first M2 slice is two Sets, mixed, through a tone mapper | 2026-07-30 | accepted |
| [ADR-0037](0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md) | Tone mapping is a uniform, and the default is chosen by looking | 2026-07-30 | accepted |
| [ADR-0038](0038-a-deck-of-one-is-bit-identical-to-a-bare-set.md) | A deck of one is bit-identical to a bare Set | 2026-07-30 | accepted |
| [ADR-0039](0039-the-artifacts-exposure-returns-to-one-as-a-convention.md) | The artifact's exposure returns to 1.0, as a convention | 2026-07-31 | accepted |
| [ADR-0040](0040-a-gain-of-zero-means-no-contribution-so-the-slot-is-skipped.md) | A gain of zero means no contribution, so the slot is skipped | 2026-07-31 | accepted |
| [ADR-0041](0041-a-candidate-is-judged-only-on-frames-it-contributed-to.md) | A candidate is judged only on the frames it contributed to | 2026-07-31 | accepted |
| [ADR-0042](0042-a-silently-wrong-image-loses-to-a-loud-failure.md) | A silently wrong image loses to a loud failure | 2026-07-31 | accepted |
| [ADR-0043](0043-the-meter-never-waits-and-the-deck-owns-it.md) | The meter never waits, and the deck owns it | 2026-07-31 | accepted |
| [ADR-0044](0044-a-test-that-survives-mutation-is-not-a-test.md) | A test that survives mutation is not a test | 2026-07-31 | accepted |
| [ADR-0045](0045-the-cli-is-an-instrument-not-a-demo.md) | The CLI is an instrument, not a demo | 2026-07-31 | accepted |
| [ADR-0046](0046-a-flag-writes-into-the-record-it-does-not-invent-one.md) | A flag writes into the record; it does not invent one | 2026-07-31 | accepted |
| [ADR-0047](0047-a-binding-blends-on-confidence.md) | A binding blends on confidence | 2026-07-31 | accepted |
| [ADR-0048](0048-four-curves-because-a-fifth-is-a-re-parameterisation.md) | Four curves, because a fifth is a re-parameterisation | 2026-07-31 | accepted |
| [ADR-0049](0049-slot-means-two-things-and-the-clash-is-recorded.md) | `Slot` means two things, and the clash is recorded rather than resolved | 2026-07-31 | accepted |
| [ADR-0050](0050-a-declared-generator-is-certain.md) | A declared generator is certain, and a sample is not always in [0,1] | 2026-08-01 | accepted |
| [ADR-0051](0051-a-name-means-one-thing-so-the-buss-noise-entry-is-deleted.md) | A name means one thing, so the bus's `noise` entry is deleted | 2026-08-01 | accepted |
| [ADR-0052](0052-a-parameter-is-keyed-by-its-layer-and-a-collision-is-refused.md) | A parameter is keyed by its layer, and a collision is refused meanwhile | 2026-08-01 | accepted |
| [ADR-0053](0053-priming-runs-the-simulation-and-skips-rendering.md) | Priming runs the simulation and skips rendering entirely | 2026-08-01 | accepted |
| [ADR-0054](0054-the-governor-budgets-from-the-probe-and-never-touches-a-live-slot.md) | The governor budgets from the probe, and never touches a Live slot | 2026-08-01 | accepted |
| [ADR-0055](0055-a-measurement-enters-the-record-stream-raw-audio-does-not.md) | A measurement enters the record stream; raw audio does not | 2026-08-01 | accepted |
| [ADR-0056](0056-beat-lock-is-feed-forward-and-the-unmeasurable-part-is-an-offset.md) | Beat lock is feed-forward, and the unmeasurable part is an offset | 2026-08-01 | accepted |
| [ADR-0057](0057-the-transport-is-driven-by-position-not-by-tempo-and-phase.md) | The transport is driven by position, not by tempo and phase | 2026-08-01 | accepted |
| [ADR-0058](0058-closed-form-is-worth-more-for-scrubbing-than-for-priming.md) | Closed form is worth more for scrubbing than for priming | 2026-08-01 | accepted |
| [ADR-0059](0059-a-records-pointer-into-the-principles-registry-is-metadata.md) | A record's pointer into the principles registry is metadata | 2026-08-01 | accepted |
| [ADR-0060](0060-the-tempo-octave-is-folded-not-judged.md) | The tempo octave is folded, not judged | 2026-08-02 | accepted |
| [ADR-0061](0061-residency-is-requested-by-the-operator-and-effective-by-the-governor.md) | Residency is requested by the operator and effective by the governor | 2026-08-02 | accepted |
| [ADR-0062](0062-warming-and-cooling-are-transitions-not-states.md) | Warming and Cooling are transitions, not states | 2026-08-02 | accepted |
| [ADR-0063](0063-an-invariant-that-is-not-yet-true-says-so.md) | An invariant that is not yet true says so | 2026-08-02 | accepted |
| [ADR-0064](0064-a-replaced-passage-is-read-to-its-end.md) | A replaced passage is read to its end | 2026-08-02 | accepted |
| [ADR-0065](0065-transport-is-two-mechanisms-and-closed-form-is-the-seam.md) | Transport is two mechanisms, and closed form is the seam | 2026-08-03 | accepted |
| [ADR-0066](0066-a-flag-becomes-a-record-writer.md) | A flag becomes a record writer, and a Set file matches it byte for byte | 2026-08-03 | accepted |
| [ADR-0067](0067-the-session-writer-never-blocks-grows-or-silently-drops.md) | The session writer never blocks, never grows, and never silently drops | 2026-08-03 | accepted |
| [ADR-0068](0068-a-timestamp-is-checked-against-a-second-measurement-not-a-constant.md) | A timestamp is checked against a second measurement, not against a constant | 2026-08-03 | accepted |
| [ADR-0069](0069-blend-modes-are-chosen-by-the-pipeline-not-by-vocabulary.md) | Blend modes are chosen by the pipeline, not by the vocabulary | 2026-08-08 | accepted |
| [ADR-0070](0070-a-channel-nobody-reads-is-a-free-variable.md) | A channel nobody reads is a free variable | 2026-08-08 | accepted |
| [ADR-0071](0071-there-is-no-panic-key.md) | There is no panic key | 2026-08-08 | accepted |
| [ADR-0072](0072-auditioning-adds-a-draw-and-never-a-step.md) | Auditioning adds a draw and never a step | 2026-08-10 | accepted |
| [ADR-0073](0073-a-control-surface-is-a-test-of-the-invariant.md) | A control surface is a test of the invariant, not a feature | 2026-08-10 | accepted |
| [ADR-0074](0074-a-plugin-boundary-is-drawn-by-the-deterministic-path.md) | A plugin boundary is drawn by the deterministic path, not by the platform | 2026-08-11 | accepted |
| [ADR-0075](0075-the-plugin-abi-passes-a-handle-and-only-what-crosses-a-process.md) | The plugin ABI passes a native handle, and only what can cross a process | 2026-08-11 | accepted |
| [ADR-0076](0076-commit-a-manifest-not-a-binary.md) | Commit a manifest, not a binary, and never fetch from build.rs | 2026-08-11 | accepted |
| [ADR-0077](0077-the-canvas-belongs-to-the-session-and-the-window-gets-no-vote.md) | The canvas belongs to the session; the window gets no vote | 2026-08-11 | accepted |
| [ADR-0078](0078-a-frame-that-is-discarded-must-not-already-have-been-recorded.md) | A frame that is discarded must not already have been recorded | 2026-08-11 | accepted |
| [ADR-0079](0079-the-status-line-is-a-display-not-a-log.md) | The status line is a display, not a log | 2026-08-11 | accepted |
| [ADR-0080](0080-the-gpl-boundary-is-a-process-and-the-protocol-is-generic.md) | The GPL boundary is a process, and the protocol is generic | 2026-08-11 | accepted |
| [ADR-0081](0081-ndjson-over-a-pipe-and-not-protobuf.md) | ndjson over a pipe, and not protobuf | 2026-08-11 | accepted |
| [ADR-0082](0082-link-is-not-automatic-and-it-is-shared.md) | Link is not automatic, and it is shared | 2026-08-11 | accepted |
| [ADR-0083](0083-mcp-is-the-only-prompt-surface.md) | MCP is the only prompt surface | 2026-08-11 | accepted |
| [ADR-0084](0084-a-procedure-change-goes-into-the-record-stream.md) | A procedure change goes into the record stream | 2026-08-12 | accepted |
| [ADR-0085](0085-a-build-carries-an-id-because-a-label-is-not-an-identity.md) | A build carries an id, because a label is not an identity | 2026-08-12 | accepted |
| [ADR-0086](0086-a-hint-says-why.md) | A hint says why | 2026-08-15 | accepted |
| [ADR-0087](0087-a-fixture-the-product-can-rewrite-is-not-a-fixture.md) | A fixture the product can rewrite is not a fixture | 2026-08-15 | accepted |
| [ADR-0088](0088-what-ships-what-you-saved-and-what-you-are-editing.md) | What ships, what you saved, and what you are editing are three places | 2026-08-15 | accepted |
| [ADR-0089](0089-history-is-gated-on-compiling-not-on-landing.md) | History is gated on compiling, not on landing | 2026-08-15 | accepted |
| [ADR-0090](0090-a-ceiling-calibrated-for-one-shape-rejects-the-next.md) | A ceiling calibrated for one shape rejects the next | 2026-08-15 | accepted |
| [ADR-0091](0091-declaration-by-absence-and-an-empty-consumes-is-a-rule.md) | Declaration by absence, and an empty `consumes` is a rule | 2026-08-15 | accepted |
| [ADR-0092](0092-a-resource-listing-is-a-curriculum-not-an-index.md) | A resource listing is a curriculum, not an index | 2026-08-15 | accepted |
| [ADR-0093](0093-a-verification-that-measures-the-wrong-tree-verifies-nothing.md) | A verification that measures the wrong tree verifies nothing | 2026-08-15 | accepted |
| [ADR-0094](0094-l2-is-stateless-as-a-rule-and-its-output-is-materialised.md) | L2 is stateless as a rule, and its output is materialised | 2026-08-16 | accepted |
| [ADR-0095](0095-a-camera-edge-is-a-gpu-buffer-and-an-l3-may-hold-state.md) | A Camera edge is a GPU buffer, and an L3 may hold state | 2026-08-16 | accepted |
| [ADR-0096](0096-camera-is-an-edge-into-l4-and-an-existing-field-is-not-a-fact.md) | Camera is an edge into L4, and an existing field is not a fact | 2026-08-16 | accepted |
| [ADR-0097](0097-overdraw-and-composition-are-different-operations.md) | Overdraw and composition are different operations, and the graph says which | 2026-08-16 | accepted |
| [ADR-0098](0098-l5-is-one-node-kind-with-two-roles.md) | L5 is one node kind with two roles | 2026-08-16 | accepted |
| [ADR-0099](0099-element-zero-is-the-oldest-living-element.md) | Element 0 is the oldest living element | 2026-08-16 | accepted |
| [ADR-0100](0100-a-published-interface-is-a-choice-of-attention.md) | A published interface is a choice of attention, not of authority | 2026-08-16 | accepted |
| [ADR-0101](0101-a-sources-number-is-recorded-not-derived.md) | A source's number is recorded, not derived | 2026-08-16 | accepted |
| [ADR-0102](0102-a-renderers-address-is-layer-and-index.md) | A renderer's address is `(layer, index)`, and `layer` alone means nothing | 2026-08-16 | accepted |
| [ADR-0103](0103-a-trailer-missed-fifteen-times.md) | A trailer missed fifteen times, because I never read my own commits | 2026-08-16 | accepted |
| [ADR-0104](0104-the-slot-contract-is-the-element-layout.md) | The slot contract is the element layout | 2026-08-17 | accepted |
| [ADR-0105](0105-field-is-a-kind-with-no-node.md) | `Field` is a kind with no node | 2026-08-18 | accepted |
| [ADR-0106](0106-an-error-scope-not-a-second-process.md) | An error scope, not a second process | 2026-08-18 | accepted |
| [ADR-0107](0107-a-chain-is-materialised-per-source.md) | A chain is materialised per source | 2026-08-18 | accepted |
| [ADR-0108](0108-cross-source-pairing-is-a-set-level-operation.md) | Cross-source pairing is a Set-level operation | 2026-08-18 | accepted |
| [ADR-0109](0109-format-the-workspace-and-split-the-gate.md) | Format the workspace, and split the gate | 2026-08-19 | **superseded by ADR-0114** |
| [ADR-0110](0110-this-machine-is-not-the-reference.md) | This machine is not the reference | 2026-08-19 | accepted |
| [ADR-0111](0111-a-name-lives-in-the-set-file-and-may-be-written-on-the-command-line.md) | A name lives in the Set file and may be written on the command line | 2026-08-19 | accepted |
| [ADR-0112](0112-what-complete-required-and-what-m3-cost.md) | What "complete" required, and what M3 cost | 2026-08-19 | accepted |
| [ADR-0113](0113-the-slot-narrowing-beat-its-estimate.md) | The slot narrowing beat its estimate, and why is the finding | 2026-08-19 | accepted |
| [ADR-0114](0114-tests-run-when-somebody-asks-not-when-git-does.md) | Tests run when somebody asks, not when git does | 2026-08-20 | accepted |
| [ADR-0115](0115-split-work-by-file-not-by-phase.md) | Split work by file, not by phase | 2026-08-20 | accepted |
| [ADR-0116](0116-stage-four-stops-claiming-the-byte-figure.md) | Stage 4 stops claiming the byte figure; the engine reports it | 2026-08-20 | accepted |
| [ADR-0117](0117-before-v1-pay-the-cost-of-changing-toward-the-ideal.md) | Before v1, pay the cost of changing toward the ideal | 2026-08-21 | accepted |
| [ADR-0118](0118-the-built-in-camera-is-a-node-unconditionally-and-last.md) | The built-in camera is a node, unconditionally and last | 2026-08-21 | accepted |
| [ADR-0119](0119-source-binds-a-uniform-and-is-read-as-a-value.md) | `Source` binds a uniform and is read as a value | 2026-08-21 | accepted |
| [ADR-0120](0120-a-record-may-reach-outside-the-stream.md) | A record may reach outside the stream, and a replay is a sandbox | 2026-08-22 | accepted |
| [ADR-0121](0121-moving-code-leaves-its-reasoning-behind.md) | Moving code leaves its reasoning behind | 2026-08-20 | accepted |
| [ADR-0122](0122-a-save-writes-the-bytes-that-are-on-screen.md) | A save writes the bytes that are on screen | 2026-08-22 | accepted |
| [ADR-0123](0123-a-fix-brief-names-the-property-not-the-shape.md) | A fix brief names the property, not the shape | 2026-08-22 | accepted |
| [ADR-0124](0124-a-save-from-mcp-does-not-wait-under-the-lock.md) | A save from MCP does not wait under the lock, and answers truthfully | 2026-08-22 | accepted |
| [ADR-0125](0125-a-client-named-id-is-an-allow-list.md) | A client-named id is an allow-list | 2026-08-22 | accepted |
| [ADR-0126](0126-a-noise-rate-follows-a-tempo-correction-and-not-a-phase-one.md) | A noise rate follows a tempo correction and not a phase one | 2026-08-02 | accepted |
| [ADR-0127](0127-the-repository-names-no-tool.md) | The repository names no tool | 2026-08-22 | **superseded by ADR-0129** |
| [ADR-0128](0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md) | A Set saved under a name the caller chose overwrites | 2026-08-22 | accepted |
| [ADR-0129](0129-a-vendor-file-is-ignored-and-removed-from-history.md) | A vendor file is ignored, and removed from history | 2026-08-22 | accepted |
| [ADR-0130](0130-a-wrapper-that-needs-a-gpu-does-not-excuse-the-decision-inside-it.md) | A wrapper that needs a GPU does not excuse the decision inside it | 2026-08-22 | accepted |
| [ADR-0131](0131-one-refusal-sentence-per-mistake-across-the-surfaces-that-face-a-person.md) | One refusal sentence per mistake, across the surfaces that face a person | 2026-08-22 | accepted |
| [ADR-0132](0132-a-rebuild-restates-the-camera-it-was-aimed-with.md) | A rebuild restates the camera it was aimed with | 2026-08-22 | accepted |
| [ADR-0133](0133-what-was-fed-to-a-generator-is-a-field-not-an-object.md) | What was fed to a generator is a field, not an object | 2026-08-22 | accepted |
| [ADR-0134](0134-the-metadata-preview-becomes-thumbnail.md) | The metadata `preview` becomes `thumbnail` | 2026-08-22 | accepted |
| [ADR-0135](0135-one-record-enum-and-one-classifier-behind-both-predicates.md) | One record enum, and one classifier behind both predicates | 2026-08-22 | accepted |
| [ADR-0136](0136-the-store-keeps-bytes-and-whoever-compiled-them-writes-the-card.md) | The store keeps bytes, and whoever compiled them writes the card | 2026-08-22 | accepted |
| [ADR-0137](0137-an-unfoldable-default-writes-the-key-absent-not-the-record-absent.md) | An unfoldable default writes the key absent, not the record absent | 2026-08-22 | accepted |
| [ADR-0138](0138-a-model-names-a-set-not-a-hash.md) | A model names a Set, not a hash | 2026-08-22 | accepted |
| [ADR-0139](0139-a-card-states-what-a-procedure-declares-and-not-what-a-set-turned-it-to.md) | A card states what a procedure declares, and not what a Set turned it to | 2026-08-22 | accepted |
| [ADR-0140](0140-a-gpu-test-lives-under-mod-gpu-and-the-rule-is-enforced-both-ways.md) | A GPU test lives under `mod gpu`, and the rule is enforced both ways | 2026-08-22 | accepted |
| [ADR-0141](0141-a-gpu-test-with-no-adapter-fails-rather-than-skipping.md) | A GPU test with no adapter fails rather than skipping | 2026-08-22 | accepted |
| [ADR-0142](0142-validation-runs-without-a-device-and-hands-build-a-plan.md) | Validation runs without a device, and hands `build` a plan | 2026-08-22 | accepted |
| [ADR-0143](0143-cargo-nextest-is-slower-here-and-was-measured-not-argued.md) | cargo-nextest is slower here, and was measured rather than argued | 2026-08-22 | **superseded by ADR-0144** |
| [ADR-0144](0144-the-test-suites-largest-cost-was-a-directory-listing.md) | The test suite's largest cost was a directory listing | 2026-08-23 | accepted |
| [ADR-0145](0145-the-storage-figure-is-computed-on-demand-not-recorded.md) | The storage figure is computed on demand, not recorded | 2026-08-23 | accepted |
| [ADR-0146](0146-a-selection-is-its-own-record-and-lands-once.md) | A selection is its own record, and lands once | 2026-08-23 | accepted |
| [ADR-0147](0147-geometry-is-not-shared-across-sets.md) | Geometry is not shared across Sets | 2026-08-23 | accepted |
| [ADR-0148](0148-a-variant-pool-is-a-set-and-the-deck-stays-a-mixer.md) | A variant pool is a Set, and the deck stays a mixer | 2026-08-23 | accepted |
| [ADR-0149](0149-source-cites-what-is-in-force-not-a-plan.md) | Source cites what is in force, not a plan | 2026-08-23 | accepted |
| [ADR-0150](0150-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once.md) | The pipeline is linear HDR, and sRGB is encoded once at final output | 2026-08-23 | accepted |
| [ADR-0151](0151-an-adr-is-a-description-of-history.md) | An ADR is a description of history | 2026-08-23 | accepted |
| [ADR-0152](0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md) | A `.kir` names a slot, and the Set names the nodes | 2026-08-21 | accepted |
| [ADR-0153](0153-a-renderer-reads-three-camera-members-and-the-l3s-five-stay-unreadable.md) | A renderer reads three camera members, and the L3's five stay unreadable | 2026-08-21 | accepted |
| [ADR-0154](0154-a-third-path-on-set-is-a-second-renderer-and-there-is-no-new-syntax.md) | A third path on `--set` is a second renderer, and there is no new syntax | 2026-08-16 | accepted |
| [ADR-0155](0155-egui-draws-the-panel-and-the-price-is-wgpu-30.md) | egui draws the panel, and the price is wgpu 30 | 2026-08-23 | accepted |
| [ADR-0156](0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md) | The console's arrangement is a tree this repository owns, not the toolkit's panels | 2026-08-23 | accepted |
| [ADR-0157](0157-a-maximum-is-honoured-and-the-leftover-is-trailing-space.md) | A maximum is honoured, and the leftover is trailing space | 2026-08-23 | accepted |
| [ADR-0158](0158-a-saved-arrangement-that-disagrees-with-itself-is-refused-not-repaired.md) | A saved arrangement that disagrees with itself is refused, not repaired | 2026-08-23 | accepted |
| [ADR-0159](0159-the-consoles-words-are-the-manuals-and-the-middle-one-is-not-a-pane.md) | The console's words are the manual's, and the middle one is not a pane | 2026-08-24 | accepted |
| [ADR-0160](0160-a-boundary-is-a-rectangle-not-a-coordinate.md) | A boundary is a rectangle, not a coordinate | 2026-08-24 | accepted |
| [ADR-0161](0161-solo-remembers-which-region-because-it-cannot-be-derived.md) | Solo remembers which region, because it cannot be derived | 2026-08-24 | accepted |
| [ADR-0162](0162-the-panels-surface-is-not-srgb.md) | The panel's surface is not sRGB | 2026-08-24 | accepted |
| [ADR-0163](0163-a-boundary-gets-first-refusal-on-a-pointer.md) | A boundary gets first refusal on a pointer | 2026-08-24 | accepted |
| [ADR-0164](0164-the-panel-is-budgeted-rather-than-forbidden-to-allocate.md) | The panel is budgeted rather than forbidden to allocate | 2026-08-24 | accepted |
| [ADR-0165](0165-the-repaint-decision-is-one-closed-list.md) | The repaint decision is one closed list, never an operation's return value | 2026-08-24 | accepted |
| [ADR-0166](0166-the-engines-frame-and-the-panels-are-one-submission.md) | The engine's frame and the panel's are one submission | 2026-08-24 | accepted |
| [ADR-0167](0167-the-panel-keeps-re-uploading-what-did-not-change.md) | The panel keeps re-uploading what did not change | 2026-08-25 | accepted |
| [ADR-0168](0168-a-backend-override-is-honoured-because-a-no-op-cannot-be-caught.md) | A backend override is honoured, because a no-op cannot be caught | 2026-08-25 | accepted |
| [ADR-0169](0169-the-timestamp-verdict-is-the-backends-not-the-machines.md) | The timestamp verdict is the backend's, not the machine's | 2026-08-25 | accepted |
| [ADR-0170](0170-a-deck-preview-cell-is-drawn-whether-or-not-a-deck-is-behind-it.md) | A deck preview cell is drawn whether or not a deck is behind it | 2026-08-25 | accepted |
| [ADR-0171](0171-the-deck-advances-and-each-sink-either-gets-the-frame-or-misses-it.md) | The deck advances, and each sink either gets the frame or misses it | 2026-08-25 | accepted |
| [ADR-0172](0172-the-frame-loop-is-the-engines-because-the-cli-is-scaffolding.md) | The frame loop is the engine's, because the CLI is scaffolding | 2026-08-25 | accepted |
| [ADR-0173](0173-a-frames-submission-is-not-only-its-sinks.md) | A frame's submission is not only its sinks | 2026-08-25 | accepted |

## Retired numbers


A superseded record keeps its row above. A **deleted principle** is recorded here, and its number is
never reused.

| Retired | Was | Replaced by | Why |
| --- | --- | --- | --- |
| P-0015 | One record tag is one record shape | [P-0031](../principles/0031-a-name-means-one-thing-across-the-system.md) | The same rule holds for any name, not only a record tag — found when `"noise"` meant two things at two confidences |
| P-0022 | Closed-form material needs no warming | [P-0032](../principles/0032-closed-form-means-scrubbable.md) | Not needing warming is the smaller half; the larger one is being scrubbable |

Re-pointing the records that cited a retired principle is permitted, and why, is
[ADR-0059](0059-a-records-pointer-into-the-principles-registry-is-metadata.md).

## Standing rules with no record yet

A principle with no ADR is one whose reasoning is still only in the code and the specification.

- [P-0005](../principles/0005-a-swap-happens-on-a-frame-boundary-and-an-over-budget-set-rolls-back-on-its-own.md) — A swap happens on a frame boundary, and an over-budget Set rolls back on its own
- [P-0067](../principles/0067-the-language-is-bounded-so-a-procedure-can-be-priced-before-it-runs.md) — The language is bounded, so a procedure can be priced before it runs. It predates the recovered history; the reasoning is in `ir-spec.md` and the check pass
- [P-0065](../principles/0065-the-signal-bus-is-not-readable-from-ir.md) — The signal bus is not readable from IR. It predates the recovered history and has no decision to record. **Renumbered from 0061 on 2026-08-23**, which two writers had reached for within three hours of each other; the earlier file keeps the number, and 0061 is [a refusal a person can reach from two surfaces is one sentence](../principles/0061-a-refusal-a-person-can-reach-from-two-surfaces-is-one-sentence.md)
- [P-0069](../principles/0069-the-three-clocks-never-collapse-into-each-other.md) — The three clocks never collapse into each other. Part of the design skeleton rather than a decision taken against an alternative: it was written down before there was an engine to test it against, and the frame path, the transport, the transitions and the build worker were all arranged around it
- [P-0070](../principles/0070-auditioning-is-a-prerequisite-not-a-convenience.md) — Auditioning is a prerequisite, not a convenience. The requirement was stated when the instrument was first described and the control was built to it; nothing was decided against an alternative. [ADR-0072](0072-auditioning-adds-a-draw-and-never-a-step.md) records how an audition behaves, not whether the control exists
