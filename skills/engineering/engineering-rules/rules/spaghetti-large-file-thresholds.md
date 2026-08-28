---
title: Treat 400-line files as large and 700-line files as enormous
section: spaghetti
scope: general
applies-to: all
status: current
tags: file-size, complexity, pr-gate, heuristic
---

## Treat 400-line files as large and 700-line files as enormous

**These thresholds are a pragmatic house heuristic — not Matt Pocock's.** He explicitly
rejects line-count measures (see `arch-deep-modules`). Use them as a cheap PR-gate
trigger only; apply `arch-deletion-test` for the real architectural judgement.

- **400 lines = large.** Before adding code, check whether the change belongs in a domain
  module, adapter, schema/settings module, or test helper. Prefer a new file over making
  a large file larger.
- **700 lines = enormous.** Only add here for a minimal localized fix, or when the net
  change demonstrably reduces complexity (e.g. extracting a block that is replaced by a
  two-line call).

**Before touching a large file, ask:**

1. Is this logic a domain concept that deserves its own module?
2. Is this a new adapter that should live in its own slice?
3. Is this schema/settings/config that belongs in a dedicated module?
4. Is this test infrastructure that belongs in a helper or fixture file?
5. Does the deletion test say this file is already earning its size, or is it a tangle
   of unrelated concerns (`spaghetti-mixed-orchestration`)?

**Direction:** when a file crosses these thresholds, prefer extracting to a domain module or
adapter slice over adding more lines. Use the thresholds as a gate to pause and check;
use `arch-deletion-test` to decide.

Reference: pragmatic house heuristic. For the architectural judgement behind
line-count signals, see `arch-deletion-test` and `arch-deep-modules`. Matt Pocock
(codebase-design) uses depth-as-leverage, not line ratios.
