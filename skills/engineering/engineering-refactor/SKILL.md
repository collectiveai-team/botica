---
name: engineering-refactor
description: Refactor a codebase toward the house engineering style, project shape, tests, and deep-module architecture. Use when the user asks to adapt a repo to these styles or practices, clean up architecture, modernize a Python project, or refactor existing code for maintainability.
---

# Engineering Refactor

Move an existing codebase toward the house engineering standard **without bulldozing local
project contracts**. The rules in `rules/` cover the Python toolchain, package layout,
deep-module architecture (Matt Pocock's vocabulary), testing, and the stack-specific
patterns (FastAPI, Prefect, LLM/AI, ASR, Next.js, k8s).

## How the rules work

This skill bundles its rules as atomic files under `rules/`. **Read `rules/_sections.md`
first** — it is the index of every rule, grouped by section, with each rule's id. Then read
the specific `rules/<id>.md` files relevant to what you are touching. `AGENTS.md` is the same
rules compiled into one document if you want to skim everything at once.

Every rule has an `applies-to` field (e.g. `all`, `nextjs`, `prefect`, `asr`) and a `status`:
- `current` — what the repos do today.
- `direction` — where new/modernized code should head.
- `legacy` — preserve in repos that use it; don't expand or fight it.

**Follow the local repo over the direction** unless the user explicitly asked to modernize.

## Workflow

### 1. Establish the baseline

Read `rules/_sections.md`, then inspect the repo's local contracts: `pyproject.toml`,
lockfiles, Makefiles, CI workflows, existing tests, package layout, `README`, `CONTEXT.md`,
and ADRs if present. Determine, citing evidence:

- Package manager + build backend (`rules/py-package-manager.md`, `rules/py-build-backend.md`).
- Toolchain era — modern uv+ruff+mypy or legacy poetry+black/isort (`rules/py-ruff-format-modern.md`, `rules/py-legacy-lint-stack.md`).
- Python version, package layout, test layout, logging, runtime entrypoints, frontend (if any).

Preserve the existing toolchain unless the user asked to migrate it (`rules/general-respect-local-repo.md`).

### 2. Find refactor targets

Read the architecture rules — `rules/arch-deep-modules.md`, `rules/arch-deletion-test.md`,
`rules/arch-adapter-discipline.md` — and the spaghetti rules (`rules/spaghetti-*.md`). Look for:

- Shallow wrappers whose interface is nearly as complex as their implementation (apply the **deletion test**).
- Logic spread across callers that should live behind one module interface.
- Files over 400 lines receiving unrelated behavior; files over 700 lines (`rules/spaghetti-large-file-thresholds.md`).
- Orchestration mixing transport, validation, domain logic, persistence, formatting (`rules/spaghetti-mixed-orchestration.md`).
- Slices that don't match the conventions (`rules/pylayout-meta-slice.md`, `rules/pylayout-adapters-slices.md`, `rules/log-get-logger-rich.md`).
- Tests asserting internals instead of behavior at the interface (`rules/test-through-interface.md`).

Use the exact vocabulary from `rules/arch-vocabulary.md`: module, interface, implementation,
depth, seam, adapter, leverage, locality.

### 3. Refactor in small slices

- Keep behavior stable unless the user asked for a behavior change.
- Move code toward domain-named modules / existing package slices; replace shallow layers with one deeper module rather than adding another abstraction (`rules/arch-adapter-discipline.md`).
- Add or update tests at the module interface before risky movement (`rules/test-in-memory-adapters.md`).
- No formatting churn outside touched code (`rules/py-no-formatter-churn.md`).
- Keep public imports and entrypoints compatible unless the task includes a breaking change.
- For stack-specific code, pull the matching rules: `rules/prefect-*.md`, `rules/llm-*.md`, `rules/asr-*.md`, `rules/api-*.md`, `rules/fe-*.md`, `rules/k8s-*.md`.

### 4. Verify

Run the narrowest useful checks first, then broaden (`rules/pr-run-checks.md`). Note that
Makefiles here are docker-only — run linters/pytest directly (`rules/infra-makefile-docker.md`).

- House stack: `ruff check`, `ruff format --check`, `pyrefly check`, `ast-grep scan`, `pytest`
  (via `uv run` / `uvx`), or the whole hook set with `uvx prek run --all-files`.
- Legacy repos: run what the repo actually has (`black`, `isort`, `pylint`/`flake8`, `mypy`,
  `pytest`) — don't introduce the house stack mid-refactor (`rules/py-legacy-lint-stack.md`).
- Always run relevant pytest tests for behavior changes; report checks that could not run.

## Output

When finished, report: what changed, which rules drove each change (cite rule ids), which
tests/checks ran and their results, and any remaining architecture risks (large files,
shallow modules, missing tests) left behind.
