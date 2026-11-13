---
role: performance
applies:
  paths: []
  tiers: [medium, high]
stages: [local, ci]
blocking: P1
max-findings: 6
---

## Invariants

<!-- Name the paths that run often or on large data, the sizes that matter (rows, files, requests
per second) and the benchmarks that cover them. -->

- A query, a request, a read of the disk or a process does not run once per item of an input that
  grows, where one call for all of them would do.
- A read of input that a user controls has a bound.

## Check always

- A query, request or disk read inside a loop.
- A blocking call on an async path.
- A cache with no bound or no invalidation.
- A limit, a timeout or a benchmark removed or loosened.

## Do not report

- A micro-optimisation with no cost at the sizes above.
- What `botica lint` already reports (`botica rules --all --select BTASYNC`).

<!-- botica:covered -->
<!-- /botica:covered -->

## Evidence

Each finding gives the count of calls and what it grows with, or the benchmark that a person would
run to measure it.
