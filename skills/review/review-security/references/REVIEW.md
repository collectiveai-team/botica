---
role: security
applies:
  paths: []
  tiers: [low, medium, high]
stages: [local, ci]
blocking: P1
max-findings: 10
---

## Invariants

- A caller reads and writes only the data its identity allows.
- A secret never reaches the repository, a log, an error message or a client.
- Input from outside the process never reaches a shell, a query, a path or a template unchecked.

## Check always

- Each new entry point checks identity and permission before it acts.
- A workflow of CI triggered by `pull_request_target` or `workflow_run` does not run code of the PR
  with secrets.
- A new dependency, action or download is pinned.

## Do not report

- What `botica lint`, zizmor, osv-scanner or dependency-review already report.
- A theoretical attack that needs access the attacker would already have.

## Evidence

Each finding gives the path from the input to the use, with file and line at each step, and the
request or file that triggers it.
