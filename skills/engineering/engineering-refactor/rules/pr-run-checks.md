---
title: Run and honestly report the repo's checks before opening a PR
section: pr
scope: general
applies-to: all
status: current
tags: pr, ci, testing, linting, checks, verification
---

## Run and honestly report the repo's checks before opening a PR

Run every check that is practical for the diff and report results truthfully. A check
that was not run must be named as such — do not imply it passed. A failing check blocks
the PR until fixed or explicitly acknowledged with a reason.

**House stack (ruff + pyrefly + ast-grep via prek + pytest):**

```bash
uv run ruff check .
uv run ruff format --check .
uvx pyrefly check --config pyproject.toml
uvx --from ast-grep-cli ast-grep scan
uv run pytest      # or: uv run pytest tests/path/to/relevant/

# Or run the whole hook set the way CI does:
uvx prek run --all-files --show-diff-on-failure
```

**Legacy repos (black + isort + pylint/flake8 + mypy + pytest):**

```bash
# Run what the repo actually has — don't introduce the house stack mid-PR
black --check .
isort --check-only .
pylint src/        # or: flake8
mypy src/
pytest
```

**Important:** Makefiles here hold docker orchestration targets, not lint/test runners
(`infra-makefile-docker`). Run linters and pytest directly — do not assume `make test`
or `make lint` exists or does what it sounds like.

**Prefer (honest partial report):**

```markdown
## Checks run
- `ruff check .` — 0 errors
- `pytest tests/test_order.py` — 12/12 passed
- Full `pytest` suite: not run (requires live DB; covered in CI)
```

**Avoid:**

```markdown
## Checks
All passing ✓
# (when only one file was linted and no tests were run)
```

Narrow test runs are acceptable when broader ones need infrastructure. Name the boundary.
Never claim a check passed if it was skipped, errored out, or only partially scoped.

Reference: `py-ruff-format-modern`, `py-legacy-lint-stack`, `infra-makefile-docker`,
`pr-description` (the "how verified" section lives here).
