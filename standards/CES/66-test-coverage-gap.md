---
id: CES-66
title: Coverage gaps are a signal
---

## Directive

An uncovered branch is investigated, and the coverage percentage is left to follow. A coverage gap
is a missing test, dead code, or a seam that cannot be tested and needs a refactor. The
investigation finds which one, and `# pragma: no cover` is not added to raise a percentage.

## Why

- A test that asserts nothing or a file left out of the report raises the percentage without
  testing anything. The gap names the line that no test runs, where a bug can stay unseen.
- A target percentage leads to tests written to run lines. Reading the gaps leads to a test of a
  behaviour or to the deletion of code that no caller needs.

## How to read a gap

| The uncovered code is | Do |
|---|---|
| a real branch that nothing tests | write the test, ideally through the interface (CES-65) |
| unreachable or dead | delete it |
| hard to reach without internals | refactor the seam so it can be driven from the outside |

## No coverage gate

No hard coverage threshold is added to reach a number, and no gap is excluded to hide it. A new gap
is read as one of the cases of the table above and handled as the table says.
