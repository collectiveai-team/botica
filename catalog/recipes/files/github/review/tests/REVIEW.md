---
role: tests
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
max-findings: 10
covered: {}
---

## Check

Check whether the changed tests fail when the behaviour is wrong. Inspect deleted assertions,
weakened thresholds and skipped checks. Tests call the public interface used in production.
Working in-memory adapters exercise the same ports as production adapters.

For each gap, name a concrete incorrect behaviour that the tests would accept and the public
entry point that reaches it. Omit only findings covered by declared checks that actually run.

<!-- botica:covered -->
<!-- /botica:covered -->
