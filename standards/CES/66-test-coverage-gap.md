---
id: CES-66
title: Coverage gaps are a signal
---

## Directive

An uncovered branch is a question and not a number. Every coverage gap is one of three things: a
missing test, dead code, or a seam that cannot be tested and wants refactoring. The gap is
investigated to find which one, and `# pragma: no cover` is not added to raise a percentage.

## Why

- A coverage percentage is easy to game (assert nothing, exclude files) and says little. The gaps
  are the signal: the specific line that nothing exercises is where a bug hides.
- Chasing a number leads to low-value tests written to touch lines. Chasing gaps leads to a real
  test or to the deletion of code that should not exist.

## How to read a gap

| The uncovered code is | Do |
|---|---|
| a real branch that nothing tests | write the test, ideally through the interface (CES-65) |
| unreachable or dead | delete it |
| hard to reach without internals | refactor the seam so it can be driven from the outside |

## No coverage gate

A hard coverage threshold is not added to win, and a gap is not suppressed to hide it. When a gap
appears, decide which of the three cases it is and act on that. Coverage is a lens on design.
