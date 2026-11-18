---
role: architecture
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
max-findings: 8
covered: {}
---

## Check

Compare new dependencies and module boundaries with this repository's documented architecture.
Inspect cycles, duplicated concepts and abstractions with one user. A function orders the steps
of a task or performs one step; domain decisions and I/O belong in named units.

Cite the documented boundary or the concept's existing name for each finding. Identify both
ends of a dependency. Omit only findings covered by declared checks that actually run.

<!-- botica:covered -->
<!-- /botica:covered -->
