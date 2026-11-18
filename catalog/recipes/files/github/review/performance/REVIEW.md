---
role: performance
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
max-findings: 6
covered: {}
---

## Check

Inspect repeated queries, growing allocations, blocking operations and unbounded caches.
Compare the cost before and after the change on a reachable path. Use sizes and timings that
this repository's requirements or measurements support.

State what grows with the input and how to measure it. Record missing scale assumptions
under `not_checked`. Omit only findings covered by declared checks that actually run.

<!-- botica:covered -->
<!-- /botica:covered -->
