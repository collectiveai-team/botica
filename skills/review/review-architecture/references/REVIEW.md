---
role: architecture
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
max-findings: 8
---

## Check always

- An import against the layers that the ADRs set.
- A new abstraction with one user.
- A new name for a concept that already has one.

## Do not report

- What `botica lint` (BTARCH001 `import-layers`) or import-linter already report.
- A refactor that the diff does not need.

## Evidence

Each finding names the modules and the import or the type, with file and line.
