---
name: engineering-pr-review
description: PR gate that applies the shared rules plus deep-module and large-file/spaghetti checks before creating, opening, updating, or marking a pull request ready. Use when an agent is about to create a PR, update a PR, publish a branch, request review, or summarize PR readiness.
---

# Engineering PR Review

Run this before creating, opening, updating, or marking a PR ready. The rules are bundled as
atomic files under `rules/`; the PR-specific gate lives in `rules/pr-*.md`.

## How the rules work

**Read `rules/_sections.md`** — the index of every rule by section. Read the specific
`rules/<id>.md` files the diff touches. `AGENTS.md` is the full compiled ruleset. Each rule
has `applies-to` (scope) and `status` (`current` / `direction` / `legacy`) — judge a diff
against what its repo actually uses, not a one-size-fits-all standard
(`rules/general-respect-local-repo.md`).

## Required context

1. Read `rules/_sections.md` and the `rules/pr-*.md` gate.
2. Inspect the diff against the target branch and the files receiving the largest additions.
3. Identify the repo's toolchain, tests, CI, package layout; read `CONTEXT.md`/ADRs if present.

## PR gate

The PR is not ready until each check has been considered and material issues are fixed or
explicitly called out.

### 1. Shared rules check

Walk the diff against the rules that match the touched files. At minimum:
- Toolchain preserved; no unrelated formatter churn (`rules/py-no-formatter-churn.md`, `rules/py-package-manager.md`).
- New dependencies justified and consistent with the repo (`rules/py-package-manager.md`).
- Settings/schemas/API/CLI/logging/adapters placed per the repo shape (`rules/pylayout-*.md`, `rules/api-*.md`, `rules/log-*.md`).
- Stack-specific rules where relevant: `rules/prefect-*.md`, `rules/llm-*.md`, `rules/asr-*.md`, `rules/fe-*.md`, `rules/k8s-*.md`.
- Tests added/updated for behavior changes; default tests need no live creds/cloud/model downloads (`rules/test-coverage-gap.md`, `rules/test-in-memory-adapters.md`).

### 2. Deep-module check

Apply `rules/arch-deletion-test.md`, `rules/arch-adapter-discipline.md`,
`rules/arch-interface-is-test-surface.md`, using the vocabulary in `rules/arch-vocabulary.md`:

- Shallow module: did the PR add a wrapper whose interface is nearly as complex as its implementation?
- Deletion test: if the new module were deleted, would complexity vanish (pass-through) or spread back across callers (earning its place)?
- Adapter discipline: a new seam with only one adapter is just indirection — remove it or justify the second adapter.
- Interface as test surface: do tests exercise observable behavior through the same seam callers use?

### 3. Spaghetti & large-file check

Per `rules/spaghetti-*.md`:
- List touched files over 400 lines (large) and over 700 lines (enormous) — these thresholds are a house heuristic, not Matt Pocock; use them to *trigger* judgement, then apply the deletion test.
- For each large touched file, decide whether the PR makes it smaller, keeps the change narrowly localized, or piles on unrelated responsibility.
- Reject changes that add branches to mixed orchestration without improving the seam (`rules/spaghetti-mixed-orchestration.md`).
- Watch for duplicated conditionals across callers, boolean-flag proliferation, catch-all `utils.py` (`rules/spaghetti-no-utils-dumping.md`), and over-long functions (`rules/spaghetti-function-size.md`).

### 4. Verification check

Per `rules/pr-run-checks.md`: run the relevant checks for the diff (linters/pytest directly —
Makefiles here are docker-only, `rules/infra-makefile-docker.md`). If only narrow tests are
practical, run those and name the broader checks not run. Never claim a check passed if it
was not run.

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
(`rules/pr-complexity-check.md`, `rules/pr-description.md`).
