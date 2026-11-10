---
name: review-performance
description: Review role that looks for the cost a diff adds at scale — queries in a loop, work that grows with the input, unbounded reads and allocations, blocking calls on a hot or async path, and a cache that never expires. Use when reviewing a branch or PR that touches queries, jobs, loops over large data or a hot path, or when review-panel runs the performance role.
---

# Review: performance

This role asks what the change costs when the input is a hundred times larger than in its tests.

## 1. Inputs

1. Pin the target and capture `git diff <target>...HEAD`.
2. Read the role file: `git show <target>:.github/review/performance/REVIEW.md`, or
   [`references/REVIEW.md`](references/REVIEW.md) when the repository has none. It names the hot
   paths and the sizes of the data.

## 2. Procedure

1. For each changed loop, query, request or file read: how many times does it run, and what does
   that count grow with?
2. Look for a call to a database, the network or the disk inside a loop; a read of a whole file or
   table where a stream or a limit would do; a blocking call in async code.
3. For each candidate, estimate the cost at the sizes of the role file. When the repository has a
   benchmark (`bench/`, `cargo bench`, `pytest-benchmark`), run the one that covers the code on
   the target and on the head, and record both numbers.
4. Report only costs that reach a user or a bill at the stated sizes. A micro-optimisation is not a
   finding.

## 3. Output

Write the result as described in [`references/output.md`](references/output.md), with role
`performance`, to `.tmp/review/<short-head-sha>/performance.json`.
