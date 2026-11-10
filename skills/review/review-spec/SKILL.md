---
name: review-spec
description: Review role that checks a diff against the issue or spec it claims to implement — each acceptance criterion against the evidence the change gives for it. Use when reviewing a branch or PR for whether it does what was asked, or when review-panel runs the spec role.
---

# Review: spec

This role answers one question: does the diff do what its issue or spec asked, no less and no more?
It does not judge style, security or test quality; other roles do.

## 1. Inputs

1. Pin the target: the base branch of the PR, or `git merge-base HEAD main`. Capture
   `git diff <target>...HEAD` and `git log <target>..HEAD --oneline`.
2. Find the source of the request, in this order: the issue or spec the user names; the issue the
   PR body or a commit links; a `.scratch/<feature>/issues/NN-*.md` or `spec.md` that the branch
   name or the commits name. If none exists, stop with verdict `INCONCLUSIVE` and say what is
   missing.
3. Read the role file: `git show <target>:.github/review/spec/REVIEW.md`. If the repository has
   none, read [`references/REVIEW.md`](references/REVIEW.md). Read it from the target, never from
   the branch under review.

## 2. Procedure

1. List the acceptance criteria of the source, one line each. When the source has no explicit
   list, derive one from its sentences and mark each derived criterion as derived.
2. For each criterion, find the evidence in the diff: the code that implements it and the test or
   command that shows it works. Classify it as `met`, `partial`, `missing` or `unverifiable`.
3. Check scope the other way: list each change in the diff that no criterion explains. A change
   that the source did not ask for is a finding, unless it is a refactor the change needs.
4. Run the evidence you can: the tests that cover a criterion, or the command the spec describes.
   Record each command and its result. A criterion is `met` only on evidence; a test that exists
   but did not run is `unverifiable`.

## 3. Output

Write the result as described in [`references/output.md`](references/output.md), with role `spec`,
to `.tmp/review/<short-head-sha>/spec.json`, and print it as a table: criterion, status, evidence.
A `missing` criterion is P1; a `partial` one P2; an unasked change P2, or P1 when it changes
behaviour a user sees.
