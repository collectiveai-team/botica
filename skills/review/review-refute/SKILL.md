---
name: review-refute
description: Review role that tries to refute each finding of the other review roles — whether it reproduces, whether its path is reachable, and whether the diff introduced it — and keeps only the findings that survive. Use after the other review-<role> skills ran, or when review-panel reaches its last step.
---

# Review: refute

This role runs last, in a fresh context, and reads only the diff and the findings. Its job is to
disprove each finding. A finding that survives a real attempt to refute it goes to the person; one
that does not is dropped with its reason.

## 1. Inputs

1. Pin the target and capture `git diff <target>...HEAD`.
2. Read every `.tmp/review/<short-head-sha>/*.json` except `refute.json`.
3. Read the role file: `git show <target>:.github/review/refute/REVIEW.md`, or
   [`references/REVIEW.md`](references/REVIEW.md) when the repository has none.

Do not read the reasoning of the other roles beyond their JSON. If you ran one of them in this
session, say so in `not_checked`: the refutation is weaker.

## 2. Procedure

For each finding, in order of severity:

1. **Introduced:** does the line exist on `<target>` already, with the same behaviour? Check with
   `git show <target>:<path>` or `git blame`. If the diff did not introduce or expose it, it is
   `pre-existing`.
2. **Reachable:** follow the callers of the code from an entry point. If no input reaches it, it is
   `unreachable`.
3. **Reproduces:** run the `how_to_verify` steps of the finding. If they pass when the finding
   says they fail, it is `not-reproduced`. If the finding has no steps, try to write the smallest
   one.
4. **Covered:** if `botica lint`, a hook or a job of CI already reports it, it is `duplicate`.
5. Otherwise it is `confirmed`. Keep a finding as `confirmed` when you could not decide; refute
   only on evidence.

## 3. Output

Write [`references/output.md`](references/output.md) with role `refute` to
`.tmp/review/<short-head-sha>/refute.json`. Each finding there is one of the inputs, with its `id`
kept, a new field `refutation` (`confirmed`, `pre-existing`, `unreachable`, `not-reproduced` or
`duplicate`) and the evidence for it. The verdict is `FAIL` when a confirmed finding reaches the
blocking severity of its own role.
