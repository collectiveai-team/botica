---
title: Keep each PR to one coherent intent
section: pr
scope: general
applies-to: all
status: current
tags: pr, review, focus, formatter
---

## Keep each PR to one coherent intent

A PR should be small enough that a reviewer can hold its entire purpose in their head.
"Small" is not measured in files or lines — a one-line semantic change can be hard to
review, and a 30-file rename can be trivial. The test is: can you state the intent in one
sentence without "and"? If not, split.

**Prefer:**

```
# One PR: "Extract order validation into OrderValidator"
# Another PR: "Fix null-check bug in order.total"
```

**Avoid:**

```
# Same PR: reformatted 40 files + added a feature + fixed an unrelated bug
# Reviewer cannot tell which diffs are meaningful vs noise
```

Split when a PR contains:

- A formatting or whitespace sweep alongside any behavior change (see `py-no-formatter-churn`)
- Two unrelated features or fixes bundled for convenience
- A refactor that is not a prerequisite for the feature in the same PR

Mixed-intent PRs are the primary cause of slow reviews and missed bugs. The formatter-churn
pattern — reformatting files outside the touched area — is the most common offender.
See `py-no-formatter-churn`.

Reference: see `pr-description` for what the description of a focused PR must contain, and
`pr-complexity-check` for the complexity gate before opening.
