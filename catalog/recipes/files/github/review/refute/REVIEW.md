---
role: refute
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
max-findings: 50
covered: {}
---

## Check

Read the completed independent role results. For each finding, inspect whether the diff
introduced it, a production entry point reaches it and an existing check covers it.
Record contrary evidence without discarding the original finding.

Only a deterministic duplicate may be removed, with a retained target whose blocking effect
is at least as strong. Keep every source finding and record unverified claims explicitly.

<!-- botica:covered -->
<!-- /botica:covered -->
