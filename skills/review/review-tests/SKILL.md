---
name: review-tests
description: Review role that checks whether the tests of a diff would fail when the change is wrong — deleted or weakened tests, assertions that cannot fail, lowered thresholds, skipped or silenced checks, and changed code with no test. Use when reviewing a branch or PR for test quality, or when review-panel runs the tests role.
---

# Review: tests

This role asks whether the tests of the change protect it: if the new code were wrong, would a
test fail?

## 1. Inputs

1. Pin the target and capture `git diff <target>...HEAD`, and the list of test files and source
   files it touches.
2. Read the role file: `git show <target>:.github/review/tests/REVIEW.md`, or
   [`references/REVIEW.md`](references/REVIEW.md) when the repository has none. Read it from the
   target, never from the branch under review.
3. Run `botica rules --all --select BTTEST` and `botica explain <slug>` for the rules the diff
   could touch. botica checks the active ones; this role reports only what they do not.

## 2. Procedure

1. Run the `test-smell-review` skill on the test files the diff adds or changes. Take its findings
   as they are; do not judge the same tests again.
2. Look for what weakens the suite outside the test bodies: a deleted or skipped test, an
   assertion made looser, a coverage threshold or a mutation threshold lowered, `|| true`,
   `continue-on-error`, a check removed from CI or from a hook, a test marked flaky.
3. For each source file with new or changed behaviour, find the test that would fail if that
   behaviour were wrong. Name it, or report that none exists.
4. Pick the most important new behaviour, break it by hand in the working tree (flip a condition,
   return early), run its test, and restore the file with `git checkout -- <file>`. Record whether
   the test failed. When a mutation report exists (`mutants.out/outcomes.json`), read it instead.

## 3. Output

Write the result as described in [`references/output.md`](references/output.md), with role
`tests`, to `.tmp/review/<short-head-sha>/tests.json`. Leave the working tree as you found it.
