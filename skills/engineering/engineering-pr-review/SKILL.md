---
name: engineering-pr-review
description: PR gate that applies the shared rules plus deep-module and large-file/spaghetti checks before creating, opening, updating, or marking a pull request ready. Use when an agent is about to create a PR, update a PR, publish a branch, request review, or summarize PR readiness.
---

# Engineering PR Review

Run this before creating, opening, updating, or marking a PR ready.

## Required: the ruleset

This skill carries the **gate**, not the rules. The rules live in the `engineering-rules`
skill.

**Invoke `engineering-rules` first** and follow its reading protocol: read its index, then the
specific rules the diff touches. Every rule id cited below resolves through it.

> If `engineering-rules` is not available, **stop and say so**. Do not run the gate from
> memory — a rule recited from memory is how a standard drifts. Install it with
> `npx skills add collectiveai-team/scaffolding --skill engineering-rules`.

## Required context

1. Read the `engineering-rules` index and the `pr-*` gate rules.
2. Inspect the diff against the target branch and the files receiving the largest additions.
3. Identify the repo's toolchain, tests, CI, package layout; read `CONTEXT.md`/ADRs if present.

## PR gate

The PR is not ready until each check has been considered and material issues are fixed or
explicitly called out.

### 1. Shared rules check

Walk the diff against the rules that match the touched files. At minimum:
- Toolchain preserved; no unrelated formatter churn (`py-no-formatter-churn`, `py-package-manager`).
- New dependencies justified and consistent with the repo (`py-package-manager`).
- Settings/schemas/API/CLI/logging/adapters placed per the repo shape (`pylayout-*`, `api-*`, `log-*`).
- Stack-specific rules where relevant: `prefect-*`, `llm-*`, `asr-*`, `fe-*`, `k8s-*`.
- Tests added/updated for behavior changes; default tests need no live creds/cloud/model downloads (`test-coverage-gap`, `test-in-memory-adapters`).

Judge each rule against what the repo actually uses, not a one-size-fits-all standard
(`general-respect-local-repo`).

### 2. Deep-module check

Apply `arch-deletion-test`, `arch-adapter-discipline`, `arch-interface-is-test-surface`, using
the vocabulary in `arch-vocabulary`:

- Shallow module: did the PR add a wrapper whose interface is nearly as complex as its implementation?
- Deletion test: if the new module were deleted, would complexity vanish (pass-through) or spread back across callers (earning its place)?
- Adapter discipline: a new seam with only one adapter is just indirection — remove it or justify the second adapter.
- Interface as test surface: do tests exercise observable behavior through the same seam callers use?

### 3. Spaghetti & large-file check

Per the `spaghetti-*` rules:
- List touched files over 400 lines (large) and over 700 lines (enormous) — these thresholds are a house heuristic, not Matt Pocock; use them to *trigger* judgement, then apply the deletion test.
- For each large touched file, decide whether the PR makes it smaller, keeps the change narrowly localized, or piles on unrelated responsibility.
- Reject changes that add branches to mixed orchestration without improving the seam (`spaghetti-mixed-orchestration`).
- Watch for duplicated conditionals across callers, boolean-flag proliferation, catch-all `utils.py` (`spaghetti-no-utils-dumping`), and over-long functions (`spaghetti-function-size`).

### 4. Verification check

Per `pr-run-checks`: run the relevant checks for the diff (linters/pytest directly — Makefiles
here are docker-only, `infra-makefile-docker`). If only narrow tests are practical, run those
and name the broader checks not run. Never claim a check passed if it was not run.

## Required PR summary

Before publishing, produce a short readiness summary:

```md
Engineering PR gate:
- Shared rules: pass/fail with notes (cite rule ids)
- Deep-module check: pass/fail with notes
- Large-file/spaghetti check: pass/fail with touched large files
- Verification: commands run and results
- Residual risks: anything reviewers should inspect
```

If any item fails, fix it before creating the PR unless the user explicitly accepts the risk
(`pr-complexity-check`, `pr-description`).
