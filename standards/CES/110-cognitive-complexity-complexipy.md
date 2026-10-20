---
id: CES-110
title: Cognitive complexity
---

## Directive

The
[Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/) score of a
function is at most 15, checked by
[`complexipy --max-complexity-allowed 15`](https://github.com/rohaquinlop/complexipy).

## Why

Ruff's `C90` (mccabe) counts branches; cognitive complexity also counts nesting and breaks in the
control flow, so a function that grows nested special cases fails it while `C90` stays clean. Both
stay on.
