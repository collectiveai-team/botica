---
title: PR description must state why, what, and how verified
section: pr
scope: general
applies-to: all
status: current
tags: pr, description, review, documentation
---

## PR description must state why, what, and how verified

A PR description is the reviewer's first read and the future archaeology layer.
Three questions must be answered: why does this change exist, what was changed, and how
was it verified? A description that omits any of these wastes reviewer time and makes
post-merge debugging harder.

**Prefer:**

```markdown
## Why
Order totals returned `None` when discount was 100% — division by zero in `order.total`.

## What
- Guard clause in `Order.compute_total()` returns 0.0 when discount >= 1.0
- Unit test added for the zero-discount edge case

## Verified
- `pytest tests/test_order.py` — all 12 tests pass
- Manually triggered a 100%-discount order in staging; total showed 0.00
```

**Avoid:**

```markdown
misc fixes

# or

Updated order logic and some tests
```

The description must be written before requesting review — not after. If "how verified"
is "not yet", the PR is not ready to open (see `pr-run-checks`).

PRs without context descriptions repeatedly cause reviewers to miss intent and merge
breaking changes with no obvious rationale.

Reference: see `pr-small-by-intent` for scoping the PR before writing the description, and
`pr-run-checks` for what "verified" concretely means.
