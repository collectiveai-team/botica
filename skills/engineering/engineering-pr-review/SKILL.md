---
name: engineering-pr-review
description: Review a local vertical slice or an integrated delivery with botica and review-panel. Use at a slice review checkpoint or before creating, publishing, updating, or marking a pull request ready.
---

# Engineering PR Review

Use this at a local slice checkpoint and before publishing a delivery. Read
`botica explain pr-small-by-intent` for the delivery rule and follow
[`references/delivery.md`](references/delivery.md) for the plan and review records. The review
procedure is the one of the authored `review-panel` skill. Completing a local ticket records its
acceptance evidence; the slice checkpoint reviews the accumulated implementation.

## 1. Context

1. Choose the scope from the delivery plan: a review block for one vertical slice, or the
   integrated delivery before publication.
2. Resolve its base and head to commit IDs. Capture the full range with
   `git diff --no-ext-diff --no-textconv <base-sha> <head-sha>` and
   `git log <base-sha>..<head-sha> --oneline`. Include earlier commits of the slice, not just the
   last ticket. For publication, verify the range also covers the proposed PR diff; an ambiguous
   merge base needs a recorded decision before claiming complete review coverage.
3. Read the judgment rules of `AGENTS.md` (between `<!-- botica:rules -->` and
   `<!-- /botica:rules -->`), `GLOSSARY.md` and the ADRs under `docs/adr/` that the diff touches.

## 2. Deterministic rules

Run `botica lint` on the changed files, or the hooks of the repository with
`uvx prek run --from-ref <base-sha> --to-ref <head-sha>`. Pass paths as arguments, not as shell
source. Fix each finding
of severity error. A warning is fixed only if the diff introduced it.

For a finding you do not understand, run `botica explain <slug>`. When the rule or its standard
cites a topic, `botica refs show <slug>` prints the code it expects.

## 3. Review

For a review block, invoke `review-panel` on the pinned base-to-head range. Its plan selects the
roles from trusted destination policy, and refute runs last. Give spec the whole slice's acceptance
criteria and relevant contracts in the same JSON context file used by the plan.
Validate the results with `botica lint --reviewed <dir> --diff <base-sha> --review-head <head-sha>
--review-context <dir>/context.json`; an incomplete plan or missing result never becomes a passing gate.

For an integrated delivery, read the slice records first. Reuse evidence only for the exact
reviewed ranges whose relevant contracts remain unchanged. Review uncovered changes and changed
contracts, and check the integrated diff against the whole spec. Record why any earlier review
became stale; do not treat a collection of passing ticket checks as a slice or integration review.

Keep the round counter across handoffs. At the three-round escalation point described by
`pr-small-by-intent`, record the blocker and preserve the work for a decision. A handoff does not
reset the counter.

## 4. Verification

Run the checks the diff affects: the tests of the touched code first, then the broader suite and
type checks when the scope requires them. Name every check that did not run. Never report a check
as passed if it did not run.

## 5. Summary and body

Record the pinned range, acceptance evidence and findings at the local slice checkpoint. Before
publishing, write a readiness summary for the integrated delivery:

```md
PR gate:
- botica lint: findings fixed, findings left and why
- review-panel: selected roles, refutations and findings left open
- Review scope: pinned slice ranges, reused evidence and integration review
- Verification: commands run and results; checks not run
- Residual risks: what reviewers should inspect
```

If an item fails, record it and keep the delivery blocked from readiness. Name an explicit user
decision to accept residual risk; do not infer one from a review-round limit. Write the PR body
with the `pr` skill only at the delivery boundary, not at each local ticket checkpoint.

Credits and sources: [`references/CREDITS.md`](references/CREDITS.md).
