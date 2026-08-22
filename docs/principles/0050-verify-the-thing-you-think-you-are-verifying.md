# Verify the thing you think you are verifying

Check that the tree under test is the tree you meant. A verification once reported "140 tests pass"
while measuring `HEAD`, because the patch application before it had failed and only the last line of
the output was read. **The tell was in the number** — the suite has 147.

**What it rules out.** Reading a green result without asking what it was green against. This sits one
layer below
[P-0025](0025-a-test-meant-to-catch-something-is-run-against-the-defect.md): the tests were correct,
and the thing they ran against was not.

**Concretely, for splitting a commit:** no patch surgery. Removing a hunk invalidates the line numbers
of every hunk after it, and `--unidiff-zero` will then insert lines into unrelated code. Build each
intermediate state by removing known strings from the complete version, **asserting each removal**, and
**build before `git add`**, so what is verified is what is staged. Check out each finished commit and
run its suite.

**A view can be structurally blind.** The `Co-Authored-By` trailer went missing from fifteen commits
in a row, and every one of them was "verified" with `git log --oneline -1` — a hash and a subject,
which is a view a trailer cannot appear in. Fifteen checks, none of which could have caught it. Ask
what the check is unable to see.

**And beware a redirection that outlives its command.** `git cat-file … > file` creates the file before
git can fail; recovery came from `git fsck --unreachable`, where a stash made with
`--include-untracked` keeps the untracked tree as its **third parent**.

**Two causes for one symptom.** "The test did not go red" can mean the mutant survived *or* that it
never compiled — an injection loop that counts failing tests cannot tell a finding from a measurement
error, so it checks.

**Where it holds.** Decided in
[ADR-0093](../adr/0093-a-verification-that-measures-the-wrong-tree-verifies-nothing.md),
[ADR-0102](../adr/0102-a-renderers-address-is-layer-and-index.md) and
[ADR-0103](../adr/0103-a-trailer-missed-fifteen-times.md).
