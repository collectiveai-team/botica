---
name: engineering-refactor
description: Restructure modules, consolidate shallow wrappers, align package layout, and add tests at module interfaces to move a codebase toward the house stack (uv + ruff + pyrefly) and deep-module architecture. Use when the user asks to refactor, restructure, reorganize, clean up architecture, tidy up code, modernize a Python project, or adapt a repo to these practices.
---

# Engineering Refactor

Move an existing codebase toward the house engineering standard **without bulldozing local
project contracts**.

## Required: the ruleset

This skill carries the **workflow**, not the rules. The rules live in the `engineering-rules`
skill — the Python toolchain, package layout, deep-module architecture (Matt Pocock's
vocabulary), testing, and the stack-specific patterns (FastAPI, Prefect, LLM/AI, ASR, Next.js,
k8s).

**Invoke `engineering-rules` first** and follow its reading protocol. Every rule id cited below
resolves through it — including its `applies-to` / `status` fields, which decide whether a rule
applies to this repo at all.

> If `engineering-rules` is not available, **stop and say so**. Do not refactor from a
> remembered standard. Install it with
> `npx skills add collectiveai-team/botica --skill engineering-rules`.

**Follow the local repo over the direction** unless the user explicitly asked to modernize.

## Workflow

### 1. Establish the baseline

Read the `engineering-rules` index, then inspect the repo's local contracts: `pyproject.toml`,
lockfiles, Makefiles, CI workflows, existing tests, package layout, `README`, `CONTEXT.md`,
and ADRs if present. Determine, citing evidence:

- Package manager + build backend (`py-package-manager`, `py-build-backend`).
- Toolchain era — the house stack (uv + ruff + ruff-format + pyrefly + ast-grep via prek) or a legacy poetry+black/isort/mypy setup (`py-ruff-format-modern`, `py-legacy-lint-stack`).
- Python version, package layout, test layout, logging, runtime entrypoints, frontend (if any).

Preserve the existing toolchain unless the user asked to migrate it (`general-respect-local-repo`).

### 2. Find refactor targets

Read the architecture rules — `arch-deep-modules`, `arch-deletion-test`,
`arch-adapter-discipline` — and the `spaghetti-*` rules. Look for:

- Shallow wrappers whose interface is nearly as complex as their implementation (apply the **deletion test**).
- Logic spread across callers that should live behind one module interface.
- Files over 400 lines receiving unrelated behavior; files over 700 lines (`spaghetti-large-file-thresholds`).
- Orchestration mixing transport, validation, domain logic, persistence, formatting (`spaghetti-mixed-orchestration`).
- Slices that don't match the conventions (`pylayout-meta-slice`, `pylayout-adapters-slices`, `log-get-logger-structlog`).
- Tests asserting internals instead of behavior at the interface (`test-through-interface`).

Use the exact vocabulary from `arch-vocabulary`: module, interface, implementation, depth,
seam, adapter, leverage, locality.

### 3. Refactor in small slices

- Keep behavior stable unless the user asked for a behavior change.
- Move code toward domain-named modules / existing package slices; replace shallow layers with one deeper module rather than adding another abstraction (`arch-adapter-discipline`).
- Add or update tests at the module interface before risky movement (`test-in-memory-adapters`).
- No formatting churn outside touched code (`py-no-formatter-churn`).
- Keep public imports and entrypoints compatible unless the task includes a breaking change.
- For stack-specific code, pull the matching rules: `prefect-*`, `llm-*`, `asr-*`, `api-*`, `fe-*`, `k8s-*`.

### 4. Verify

Run the narrowest useful checks first, then broaden (`pr-run-checks`). Note that Makefiles here
are docker-only — run linters/pytest directly (`infra-makefile-docker`).

- House stack: `ruff check`, `ruff format --check`, `pyrefly check`, `ast-grep scan`, `pytest`
  (via `uv run` / `uvx`), or the whole hook set with `uvx prek run --all-files`.
- Legacy repos: run what the repo actually has (`black`, `isort`, `pylint`/`flake8`, `mypy`,
  `pytest`) — don't introduce the house stack mid-refactor (`py-legacy-lint-stack`).
- Always run relevant pytest tests for behavior changes; report checks that could not run.

## Output

When finished, report: what changed, which rules drove each change (cite rule ids), which
tests/checks ran and their results, and any remaining architecture risks (large files,
shallow modules, missing tests) left behind.
