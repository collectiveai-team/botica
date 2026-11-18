---
role: security
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
max-findings: 10
covered: {}
---

## Check

Trace changed inputs through authorization and tenant boundaries. Inspect secret handling,
injection, path access and dependency changes. A workflow with privileged credentials must
use trusted code and instructions.

Describe the reachable attack, its entry point and the affected operation. Name a credential
by its kind and location, never its value. Omit only findings covered by declared checks
that this repository actually runs.

<!-- botica:covered -->
<!-- /botica:covered -->
