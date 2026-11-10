---
role: spec
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
max-findings: 10
---

## Check always

- Each acceptance criterion of the issue has code and evidence in the diff.
- The PR body or the commits link the issue.
- The diff changes nothing that the issue does not explain.

## Do not report

- Style, naming and formatting.
- A finding of another role (security, tests, architecture).
- A criterion that the issue marks as out of scope or for a later issue.

## Evidence

Each finding quotes the criterion and points at the file and line, or says that no line exists.
