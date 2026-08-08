---
title: Audit added complexity before opening or updating a PR
section: pr
scope: general
applies-to: all
status: current
tags: pr, complexity, architecture, review, deps, testing
---

## Audit added complexity before opening or updating a PR

Before the PR opens (or a new commit is pushed), run a five-point complexity audit on the
diff. Each point is either resolved or explicitly called out in the PR description. A PR
that silently adds complexity in any of these dimensions is not ready for review.

**The five checks:**

1. **Shallow modules** — apply the deletion test (`arch-deletion-test`) to every new class
   or function. If deleting it would change nothing but an import, it should not exist.

2. **Spaghetti conditionals** — new files that mix transport, validation, domain logic,
   persistence, and formatting in one place fail `spaghetti-mixed-orchestration`. Split
   before opening.

3. **Large-file growth** — list every touched file that crosses 400 lines (large) or 700
   lines (enormous) after the change (`spaghetti-large-file-thresholds`). Name them in the
   PR description. Growth past 700 lines requires justification or a split.

4. **New dependency surface** — new third-party packages added to `pyproject.toml` or
   `package.json` must be justified. Check whether an existing dep already covers the need
   (`py-package-manager`). One well-used library beats two half-used ones.

5. **Tests that miss the seam** — new tests must assert behavior at the public interface,
   not internal state (`test-through-interface`). Tests that reach into private methods or
   mock implementation details do not count as coverage.

**Prefer (PR description includes the audit):**

```markdown
## Complexity audit
- Deletion test: `EmailFormatter` hides 3 rendering strategies — earns its place
- Large files: `notification_service.py` goes from 380 → 430 lines (large but not enormous)
- New deps: none
- Tests: all assertions go through `NotificationService.send()`
```

**Avoid:**

Opening a PR that adds a 900-line file, three new packages, and tests that only mock
internal methods — with no acknowledgment of any of it.

Reference: `arch-deletion-test`, `spaghetti-mixed-orchestration`, `spaghetti-large-file-thresholds`,
`py-package-manager`, `test-through-interface`.
