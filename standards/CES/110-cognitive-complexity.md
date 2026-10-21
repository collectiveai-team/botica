---
id: CES-110
title: Cognitive complexity
---

## Directive

The
[Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/) score of a
function is at most 15. A function nested in another adds to the one that holds it, as
[complexipy](https://github.com/rohaquinlop/complexipy) counts it. It applies to Python,
TypeScript, Go and Rust.

## Why

Ruff's `C90` (mccabe) counts branches, which is cyclomatic complexity. Cognitive complexity
penalizes nesting depth and breaks in control flow, and it tracks how hard a person finds a
function to read. An agent that keeps patching one function with more special-cased `if` and `try`
nesting, and does not extract helpers, can stay clean for ruff while the function gets harder to
follow. The two measure different things and both stay on.
