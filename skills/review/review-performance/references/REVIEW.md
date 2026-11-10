---
role: performance
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
max-findings: 6
---

## Hot paths and sizes

<!-- Name the paths that run often or on large data, and the sizes that matter: rows, files,
requests per second. -->

## Check always

- A query, request or disk read inside a loop.
- A blocking call on an async path.

## Do not report

- A micro-optimisation with no cost at the sizes above.
- What `botica lint` already reports (`botica rules --all --select BTASYNC`).

## Evidence

Each finding gives the count of calls and what it grows with, or the numbers of a benchmark.
