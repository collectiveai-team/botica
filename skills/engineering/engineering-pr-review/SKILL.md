---
name: engineering-pr-review
description: PR gate that runs botica, reviews the diff with the upstream code-review skill and writes the body with the pr skill, before creating, opening, updating, or marking a pull request ready. Use when an agent is about to create a PR, update a PR, publish a branch, request review, or summarize PR readiness.
---

# Engineering PR Review

Run this before creating, opening, updating, or marking a PR ready. This skill is a workflow and
states no rule of its own: the rules are in botica, and the review procedure is the one of the
upstream `code-review` skill.

## 1. Context

1. Pin the target branch and capture the diff: `git diff <target>...HEAD` and
   `git log <target>..HEAD --oneline`.
2. Read `AGENTS.md` (its standards index and the judgment rules between `<!-- botica:rules -->` and
   `<!-- /botica:rules -->`), `GLOSSARY.md` and the ADRs under `docs/adr/` that the diff touches.

## 2. Deterministic rules

Run `botica lint` on the changed files (`botica lint $(git diff --name-only <target>...HEAD)`), or
the hooks of the repository with `uvx prek run --from-ref <target> --to-ref HEAD`. Fix each finding
of severity error. A warning is fixed only if the diff introduced it.

For a finding you do not understand, run `botica explain <slug>`. When the rule or its standard
cites a topic, `botica refs show <slug>` prints the code it expects.

## 3. Review

Invoke the upstream `code-review` skill against `<target>`. Its Standards axis reads the documents
of step 1, and the judgment rules of the `AGENTS.md` block are part of them. Its Spec axis compares
the diff with the issue or spec.

## 4. Verification

Run the checks the diff affects: the tests of the touched code first, then the broader suite and
type checks when the scope requires them. Name every check that did not run. Never report a check
as passed if it did not run.

## 5. Summary and body

Before publishing, write a short readiness summary:

```md
PR gate:
- botica lint: findings fixed, findings left and why
- code-review: Standards and Spec, with the findings left open
- Verification: commands run and results; checks not run
- Residual risks: what reviewers should inspect
```

If an item fails, fix it before creating the PR unless the user accepts the risk. Write the PR body
with the `pr` skill.

Credits and sources: [`references/CREDITS.md`](references/CREDITS.md).
