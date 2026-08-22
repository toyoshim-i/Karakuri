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

**And beware a redirection that outlives its command.** `git cat-file … > file` creates the file before
git can fail; recovery came from `git fsck --unreachable`, where a stash made with
`--include-untracked` keeps the untracked tree as its **third parent**.

**Where it holds.** Decided in
[ADR-0093](../adr/0093-a-verification-that-measures-the-wrong-tree-verifies-nothing.md).
