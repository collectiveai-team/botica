---
title: Never reformat outside the touched area — it buries the real diff
section: py
scope: general
applies-to: all
status: current
tags: python, formatting, git, diff, code-review
---

## Never reformat outside the touched area — it buries the real diff

Running a formatter on the entire repo (or on files you didn't logically change) swamps
the diff with noise. Reviewers can't tell intent from formatting, `git blame` is polluted,
and bisects become unreliable. Run the formatter only on touched files or the touched
package.

**Prefer:**

```bash
# Format only the files you modified
ruff format mypackage/adapters/sqs.py mypackage/adapters/gcp.py

# Or format the package, not the whole repo
ruff format mypackage/

# black equivalent
black mypackage/streaming/
```

**Avoid:**

```bash
ruff format .        # reformats notebooks/, tests/, scripts/ — noise in the diff
black .              # same problem
```

When a repo has no formatter configured at all, do not introduce formatting changes to
unrelated files — that is a migration task, not a side effect.
