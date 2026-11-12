---
role: spec
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local]
blocking: P1
max-findings: 10
---

## Invariants

- A change does what its issue asks, and nothing the issue does not explain.

## Check always

- Each acceptance criterion of the issue has code and evidence in the diff.
- The PR body or the commits link the issue.
- The diff changes nothing that the issue does not explain, except a refactor the change needs and
  the description says so.

## Do not report

- Style, naming and formatting.
- A finding of another role (security, tests, architecture).
- A criterion that the issue marks as out of scope or for a later issue.

## Evidence

Each finding quotes the criterion and points at the file and line, or says that no line exists.
