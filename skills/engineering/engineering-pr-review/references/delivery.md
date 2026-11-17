# Local delivery workflow

Read `botica explain pr-small-by-intent` for the rule. Read the project's glossary for Ticket,
Vertical slice, Review block and Delivery when those terms are defined there.

## Plan the delivery

Before `/to-spec`, `/to-tickets`, `/implement` or `/implement-spec`, locate the project's spec and
issue-tracker conventions. Create or update a delivery plan beside the spec. Preserve upstream
skill files and their version pins; use this plan as context when invoking those skills.
Run their implementation steps per ticket and schedule their closing adversarial review at the
owning slice's checkpoint. Ticket-level tests still run as the implementation changes.

1. State the delivery intent and link the spec.
2. Define slices by observable end-to-end behaviour. Assign each acceptance criterion to a slice,
   including failure behaviour and the contracts it shares with another slice.
3. Assign tickets to slices. A ticket may cover a prerequisite or one part of a behaviour; record
   its owning slice and acceptance evidence without opening a PR.
4. Record the integration branch and a base commit for each review block. If contributors use
   separate worktrees, integrate their commits before pinning the block's head for review.
5. Record any reason to split publication. When the user requests a stack, keep explicit delivery
   boundaries; do not turn every dependent commit into a PR.

Use the issue tracker's statuses for tickets. Keep slice review status and delivery readiness
separate: a completed ticket can belong to a blocked slice.

## Implement and review locally

Commit ticket changes with a pointer to the ticket or its acceptance criterion. Run the checks
affected by each change and record their results. Commit boundaries retain the history of the
work without deciding publication boundaries.

When a slice's acceptance criteria can be exercised together, pin its base and head and invoke
`engineering-pr-review` for that review block. Give reviewers the full accumulated diff, the spec,
the criteria and relevant contracts. Record each round, confirmed findings, refutations and
checks not run.

After fixes, pin the new head for the next round. Preserve the round counter across agents and
handoffs. Follow the rule's escalation procedure when three rounds leave a confirmed P0 or P1:
retain the implementation and evidence, mark the slice blocked and request a decision. Do not
automatically delete branches or worktrees.

## Check integration and publish

After integrating the reviewed slices, invoke `engineering-pr-review` in delivery scope. Map the
spec's acceptance criteria to evidence on the integrated head. Exercise contracts between slices
and review any code or contract changed since a slice's review. Keep unavailable checks explicit.

For a previously reviewed range, compare its commit IDs and relevant contracts before reusing its
evidence. A rebase, conflict resolution or later edit needs a recorded comparison; otherwise mark
the affected review stale. Run the integration checks on the final head.

Pass the spec, ticket links, slice records and integration results to `/pr`. Follow the target
repository's PR template. Do not close or reorganize existing PRs or rewrite shared branches as
part of adopting this workflow without the user's authorization.

## Delivery plan record

Use the local tracker's paths and statuses. A plan can use this shape:

```markdown
# Delivery: <one intent>

Spec: <path>
Integration branch: <branch>
Publication: one PR, or the reason for each split

## Slice: <observable behaviour>

Acceptance criteria: <spec references, including failure behaviour>
Tickets: <paths or IDs>
Contracts: <inputs, outputs, dependencies and consumers>
Review base: <commit SHA>
Reviewed head: <commit SHA>
Rounds used: <count, preserved across handoffs>
Review status: <not reviewed, blocked, reviewed or stale>
Evidence: <commands, results and review report paths>
Open findings: <IDs, severity, disposition and unresolved decisions>

## Integration

Final head: <commit SHA>
Spec coverage: <acceptance criteria and their evidence>
Cross-slice checks: <commands and results>
Reused reviews: <exact ranges and contract comparisons>
Checks not run: <commands and reasons>
Readiness: <blocked or ready, with remaining findings>
```
