---
id: 0141
title: A GPU test with no adapter fails rather than skipping
status: accepted
date: 2026-08-22
supersedes: []
superseded_by: []
principles: []
tags: [testing, workflow]
---

# A GPU test with no adapter fails rather than skipping

## Context

The workspace disagreed with itself. 280 sites `.expect(…)` and panic; seven printed a skip
and returned; one returned in silence. Meanwhile `.githooks/pre-push` stated the rule as
settled — *"There is no skip: a test that passes by not running is worse than one that
fails, which is what every GPU test in this workspace says for itself"* — which was **false
for eight tests**, and neither the rule nor the code knew it.

## Decision

All 301 acquire with `.expect`, and a machine with no adapter fails them.

**This is a behaviour change**: eight tests that used to pass on such a machine now fail
there.

The argument turns on what changed underneath. A skip was tolerable only while there was no
other way to get a run out of a machine with no device. `--skip gpu::`
([ADR-0140](0140-a-gpu-test-lives-under-mod-gpu-and-the-rule-is-enforced-both-ways.md)) is
now that way, and it is better in the way that matters: a skip at the runner is said once,
out loud, in the invocation, where a skip inside a test is a green result that measured
nothing and says so to nobody.

`pre-push`'s claim is true for the first time. Its comment now also says the filter exists
and why the gate deliberately does not use it.

## Alternatives

**Reconcile the other way — make every GPU test skip loudly when there is no adapter.** The
serious alternative, and it wins the day this workspace has to run somewhere without a GPU:
CI, a sandbox, a contributor on a VM. Rejected for now because that day has not come, and
because a suite that goes green on a broken driver by running nothing is how a release ships
broken. If it does come, the shape to adopt is a loud skip **plus a floor assertion** — at
least N GPU tests ran — or the first silent driver failure will pass a tag push having
tested nothing.

**Leave the three behaviours as they are and correct the hook's wording.** Rejected: it
documents a disagreement rather than settling one, and the disagreement was invisible for
long enough that a written rule had already gone stale against it.
