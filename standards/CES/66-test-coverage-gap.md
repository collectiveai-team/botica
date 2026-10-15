---
id: CES-66
title: coverage gaps are a signal
---

**Tier:** judgment

## Directive

Treat an uncovered branch as a **question, not a number**. Every coverage gap is one of: a
missing test, dead code, or an untestable seam that wants refactoring. Investigate which — don't
paper over it with a `# pragma: no cover` to make a percentage go up.

## Why

- A coverage *percentage* is easy to game (assert nothing, exclude files) and says little. The
  *gaps* are the signal: the specific line nothing exercises is exactly where a bug will hide.
- Chasing a number leads to low-value tests written to touch lines. Chasing gaps leads to either
  a real test or the deletion of code that shouldn't exist.

## How to read a gap

| The uncovered code is… | Do |
|---|---|
| a real branch nothing tests | write the test (ideally through the interface, CES-65) |
| unreachable / dead | delete it |
| hard to reach without internals | refactor the seam so it's drivable from the outside |

## Judgment

There is no hard coverage gate. Don't add one to "win"; don't suppress a gap to hide it. When a
gap appears, decide which of the three cases it is and act on that — coverage is a lens on design,
not a target.
